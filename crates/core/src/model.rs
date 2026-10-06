use serde::{Deserialize, Serialize};

/// The file-format identifier written in every `project.json` (see `docs/product/FILE-FORMAT.md`).
pub const FORMAT: &str = "fliphopper/1";

/// A project: one seabag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub format: String,
    pub id: String,
    pub name: String,
    /// The progress ladder for this project, lowest step first.
    #[serde(default = "default_labels")]
    pub labels: Vec<String>,
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
    /// A short note left on release for whoever picks the task up next.
    #[serde(default)]
    pub handoff_note: Option<String>,
    #[serde(default, skip_deserializing)]
    pub description: String,
}

/// A whole seabag after loading: the project, its areas and tasks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Seabag {
    pub project: Project,
    pub areas: Vec<Area>,
    pub tasks: Vec<Task>,
}
