use serde::{Deserialize, Serialize};

/// The file-format identifier written in every `project.json` (see `docs/product/FILE-FORMAT.md`).
pub const FORMAT: &str = "allomos/1";

/// The format FlipHopper wrote before the app was renamed Allomos (decision 0013, 2026-10-10).
/// Still read, so no project breaks; `alm migrate` and the app's move rewrite it as [`FORMAT`].
pub const LEGACY_FORMAT: &str = "fliphopper/1";

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
