use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The file-format identifier written in every `project.json` (see `docs/product/FILE-FORMAT.md`):
/// `allomos/<FORMAT_VERSION>`.
pub const FORMAT: &str = "allomos/3";

/// The number in [`FORMAT`]. It goes up whenever the taskbag changes shape; an older one refuses
/// a newer taskbag (Task-A01-25), and a newer one has an agent update an older taskbag by following
/// the migration entries in `crates/store/src/versions.rs` (Task-A01-32). Raising it needs an entry
/// for the version it replaces.
///
/// 2: handoff rules in `project.json`, `handoff-template.md`, `project-notes.md` and `handoffs/`
/// (Task-A03-06).
/// 3: `events/`, one file per change, the history of changes (Task-A01-17), and a task's
/// optional `finished_without_owner` (Task-A03-15).
pub const FORMAT_VERSION: u32 = 3;

/// The oldest format this Allomos can update (owner, 2026-10-14; Task-A01-32). An older taskbag
/// is told it is too old (`taskbag_too_old`); between this and [`FORMAT_VERSION`] it is read, and
/// only the update's own commands may change it until `alm migrate --finish`.
pub const OLDEST_FORMAT_VERSION: u32 = 3;

/// The number in an `allomos/<n>` format identifier; `None` for anything else (for example
/// FlipHopper's `fliphopper/1`).
pub fn format_version(format: &str) -> Option<u32> {
    let n = format.strip_prefix("allomos/")?;
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit()) || n.starts_with('0') {
        return None;
    }
    n.parse().ok()
}

/// A project: one taskbag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub format: String,
    pub id: String,
    pub name: String,
    /// The progress ladder for this project, lowest step first.
    #[serde(default = "default_labels")]
    pub labels: Vec<String>,
    /// The folder with the project's code and its AGENTS.md / CLAUDE.md, relative to the folder
    /// holding `.allomos/` (`.` when it is that folder), or absolute. Unset until the app
    /// records it (docs/product/OPEN-FOLDER-FLOW.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_folder: Option<String>,
    /// The language new and changed tasks are written in: [`TASK_LANGUAGE_UI`] (follow the
    /// Allomos app's interface language, the default) or a language code such as `zh-CN`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_language: Option<String>,
    /// When a handoff must be written (Task-A03-06). Missing: the defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<HandoffRules>,
}

/// Default quota gate: with this percentage of the 5-hour quota left, start nothing new; write the
/// handoff and stop.
pub const DEFAULT_QUOTA_GATE: u8 = 3;
/// Default context gate: at this percentage of context used, finish the current step, write the
/// handoff, commit, and give the person the exact prompt for a new session (owner, 2026-10-12).
pub const DEFAULT_CONTEXT_GATE: u8 = 90;
/// Default for when else to write a handoff.
pub const DEFAULT_WRITE_WHEN: &str = "after every meaningful step: a task finished, a finding, a commit, a process started or stopped";

/// A project's handoff rules (Task-A03-06). Every field is optional; a missing one takes its
/// default. `overrides` give an agent or account its own gates, which win over the project's.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_gate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_gate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_when: Option<String>,
    /// By agent or account name, for example `codex` or `claude/work`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, Gates>,
}

/// An agent's or account's own gates; a missing one follows the project.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gates {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_gate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_gate: Option<u8>,
}

/// The rules that apply to one agent: the project's, with its override, and the defaults filled in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppliedRules {
    pub quota_gate: u8,
    pub context_gate: u8,
    pub write_when: String,
    /// The override that applied, if any.
    pub override_for: Option<String>,
}

impl HandoffRules {
    /// The rules for `who` (a holder such as `codex/a01-03` or `claude`): an override named exactly
    /// `who`, or the longest one `who` starts with followed by `/`, wins over the project's gates.
    pub fn applied(&self, who: Option<&str>) -> AppliedRules {
        let over = who.and_then(|who| {
            self.overrides
                .iter()
                .filter(|(name, _)| who == name.as_str() || who.starts_with(&format!("{name}/")))
                .max_by_key(|(name, _)| name.len())
        });
        let pick =
            |own: Option<u8>, project: Option<u8>, default: u8| own.or(project).unwrap_or(default);
        AppliedRules {
            quota_gate: pick(
                over.and_then(|(_, g)| g.quota_gate),
                self.quota_gate,
                DEFAULT_QUOTA_GATE,
            ),
            context_gate: pick(
                over.and_then(|(_, g)| g.context_gate),
                self.context_gate,
                DEFAULT_CONTEXT_GATE,
            ),
            write_when: self
                .write_when
                .clone()
                .unwrap_or_else(|| DEFAULT_WRITE_WHEN.to_string()),
            override_for: over.map(|(name, _)| name.clone()),
        }
    }
}

/// `task_language` value meaning "the language the Allomos app shows" (Task-A01-21).
pub const TASK_LANGUAGE_UI: &str = "ui";

/// A language code Allomos accepts for `task_language`: letters, digits and `-`, starting with
/// two or three letters (`en`, `zh-CN`, `pt-BR`, `yue`), at most 35 characters.
pub fn valid_language(code: &str) -> bool {
    let mut parts = code.split('-');
    let first = parts.next().unwrap_or_default();
    code.len() <= 35
        && (2..=3).contains(&first.len())
        && first.chars().all(|c| c.is_ascii_alphabetic())
        && parts.all(|p| (1..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

pub fn default_labels() -> Vec<String> {
    [
        "implemented",
        "locally verified",
        "deployed and verified",
        "owner verified",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// An area: *what* part of the product a task belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Area {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub order: u32,
}

/// An iteration: *when* tasks are planned, a time-box that ends in a release (R89).
/// Optional for a task; a task without one is unplanned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Iteration {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub order: u32,
    /// First day, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// Last day, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// The release this iteration produces, for example `v0.2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
}

/// Where a task is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Backlog,
    Ready,
    InProgress,
    Waiting,
    Done,
    Dropped,
}

impl State {
    pub const ALL: [State; 6] = [
        State::Backlog,
        State::Ready,
        State::InProgress,
        State::Waiting,
        State::Done,
        State::Dropped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            State::Backlog => "backlog",
            State::Ready => "ready",
            State::InProgress => "in_progress",
            State::Waiting => "waiting",
            State::Done => "done",
            State::Dropped => "dropped",
        }
    }
}

/// One task. `description` comes from the task's `description.md`, not from `task.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub area: String,
    pub state: State,
    /// The highest step of the project's ladder this task has proven, if any.
    #[serde(default)]
    pub label: Option<String>,
    /// Who holds the task: a person or an agent session.
    #[serde(default)]
    pub holder: Option<String>,
    /// What blocks the next step, in plain words.
    #[serde(default)]
    pub waiting_on: Option<String>,
    #[serde(default)]
    pub next_dependency: Option<String>,
    /// The iteration the task is planned in, if any.
    #[serde(default)]
    pub iteration: Option<String>,
    /// A short note left on release for whoever picks the task up next.
    #[serde(default)]
    pub handoff_note: Option<String>,
    /// Why the task was finished without the owner's check, and in which run: set only by
    /// `alm done --without-owner` (Task-A03-15). Kept when the owner checks it later, so such tasks
    /// stay searchable; the owner's check shows in the label.
    #[serde(default)]
    pub finished_without_owner: Option<String>,
    #[serde(default, skip_deserializing)]
    pub description: String,
}

/// A whole taskbag after loading: the project, its areas, iterations and tasks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Taskbag {
    pub project: Project,
    pub areas: Vec<Area>,
    /// Sorted by order, then ID.
    pub iterations: Vec<Iteration>,
    pub tasks: Vec<Task>,
}
