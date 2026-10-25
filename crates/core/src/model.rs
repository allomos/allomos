use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Fields of a taskbag file this Allomos does not know, by name (Task-A01-piano403, approver
/// 2026-10-25). They are read past, listed among the problems as a reminder (a newer Allomos may
/// have added them, or the name is mistyped), and written back unchanged, so an older Allomos does
/// not drop what a newer one wrote. A value this Allomos does not know for a fixed-choice field
/// (a task's or iteration's `state`, an event's `kind`) is kept here too, under the field's name,
/// while the field itself reads as `Unknown` and is not written.
pub type Extra = BTreeMap<String, Value>;

/// The file-format identifier written in every `project.json` (see `docs/product/FILE-FORMAT.md`):
/// `allomos/<FORMAT_VERSION>`.
pub const FORMAT: &str = "allomos/6";

/// The number in [`FORMAT`]. It goes up whenever the taskbag changes shape. An older Allomos
/// refuses a newer taskbag (Task-A01-25); this one reads only its own format. A taskbag in any
/// older format is moved into a new taskbag by an AI agent, like another task system (owner,
/// 2026-10-15; Task-A01-45): no per-version steps are needed when the number goes up.
///
/// 2: handoff rules in `project.json`, `handoff-template.md`, `project-notes.md` and `handoffs/`
/// (Task-A03-06).
/// 3: `events/`, one file per change, the history of changes (Task-A01-17), and a task's
/// optional `finished_without_owner` (Task-A03-15).
/// 4: the `blocked` state with a task's `blocked_by`, and a waiting task's optional `who`
/// (decision 0016, Task-A01-31); the human check: `finished_without_owner` becomes
/// `without_human_check`, with optional `human_check_steps`, and the default top label is
/// "human verified" (Task-A03-18).
/// 5: an iteration's `state`: planned (the default, not written), active or closed; at most one
/// active (Task-A01-root343). Iteration files no longer refuse a field they do not know, so a later
/// optional field there needs no new format.
///
/// Since Task-A01-piano403 (approver, 2026-10-25) no file refuses a field or a value it does not
/// know: they are kept and listed (see [`Extra`]). So the number goes up only for a change an older
/// Allomos would get wrong (a file renamed or moved, an existing field's meaning changed), never for
/// a new optional field or value.
/// 6: the taskbag's project folders, each with a short name (`project_folders`, replacing
/// `project_folder`), and the folders each task belongs to (`folders`; decision 0019,
/// Task-A01-dime494). An `allomos/5` taskbag is still read and written as it is; it becomes
/// `allomos/6` only when a person asks (`alm upgrade`; approver, 2026-10-25).
pub const FORMAT_VERSION: u32 = 6;

/// The oldest format this Allomos reads and writes as it is ([`READ_FROM`]..=[`FORMAT_VERSION`]):
/// `allomos/5` stays readable after `allomos/6`, so nothing changes a taskbag until a person
/// upgrades it. Older formats are moved in by an agent (Task-A01-45).
pub const READ_FROM: u32 = 5;

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
    /// `allomos/6`: the project folders, each with a short name and a path read like
    /// `project_folder` (decision 0019). The first is the main one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_folders: Vec<ProjectFolder>,
    /// The language new and changed tasks are written in: [`TASK_LANGUAGE_UI`] (follow the
    /// Allomos app's interface language, the default) or a language code such as `zh-CN`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_language: Option<String>,
    /// When a handoff must be written (Task-A03-06). Missing: the defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<HandoffRules>,
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
}

/// One project folder of an `allomos/6` taskbag (decision 0019).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFolder {
    /// Short, unique in the taskbag: lowercase letters, digits and `-`, such as `allomos-site`.
    pub name: String,
    /// Relative to the folder holding `.allomos/` (`.` for that folder), or absolute.
    pub path: String,
    #[serde(flatten)]
    pub extra: Extra,
}

impl Project {
    /// Written in `allomos/6` or later: task folders may be used.
    pub fn upgraded(&self) -> bool {
        format_version(&self.format).is_some_and(|n| n >= 6)
    }

    /// The main project folder as recorded: the first of `project_folders`, else `project_folder`.
    pub fn main_folder(&self) -> Option<&str> {
        self.project_folders
            .first()
            .map(|f| f.path.as_str())
            .or(self.project_folder.as_deref())
            .map(str::trim)
            .filter(|p| !p.is_empty())
    }
}

/// A folder's short name as decision 0019 writes it: lowercase letters, digits and `-`, 1 to 40.
pub fn valid_folder_name(name: &str) -> bool {
    (1..=40).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
}

/// A folder name made from a folder's own name (`Allomos Site` → `allomos-site`); `main` when
/// nothing usable is left.
pub fn folder_name_from(base: &str) -> String {
    let mut out = String::new();
    for c in base.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(40).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "main".into() } else { out }
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
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
}

/// An agent's or account's own gates; a missing one follows the project.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Gates {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_gate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_gate: Option<u8>,
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
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
        "human verified",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// An area: *what* part of the product a task belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub order: u32,
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
}

/// An iteration: *when* tasks are planned, a time-box that ends in a release (R89).
/// Optional for a task; a task without one is unplanned. Unknown fields are read past, not
/// refused (approver, 2026-10-22; Task-A01-root343), and kept in `extra` (Task-A01-piano403).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Planned, active or closed (Task-A01-root343); missing means planned.
    #[serde(default, skip_serializing_if = "IterationState::not_written")]
    pub state: IterationState,
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
}

/// Where an iteration is (approver, 2026-10-22; Task-A01-root343): planned → active → closed. At
/// most one iteration is active at a time. Only a person (in the app) or an agent (`alm change
/// iteration <id> --state …`) sets it; nothing infers it from dates or progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationState {
    #[default]
    Planned,
    Active,
    Closed,
    /// A value this Allomos does not know, perhaps from a newer one; the value itself is kept in
    /// the iteration's `extra` (Task-A01-piano403), and this Allomos does not change it.
    #[serde(other)]
    Unknown,
}

impl IterationState {
    pub fn is_planned(&self) -> bool {
        *self == IterationState::Planned
    }

    /// Left out of the file: planned (the default) and an unknown value, which `extra` writes.
    pub fn not_written(&self) -> bool {
        matches!(self, IterationState::Planned | IterationState::Unknown)
    }

    /// As files and commands write it: `planned`, `active`, `closed`.
    pub fn as_str(&self) -> &'static str {
        match self {
            IterationState::Planned => "planned",
            IterationState::Active => "active",
            IterationState::Closed => "closed",
            IterationState::Unknown => "unknown",
        }
    }

    pub fn parse(text: &str) -> Option<IterationState> {
        match text.trim() {
            "planned" => Some(IterationState::Planned),
            "active" => Some(IterationState::Active),
            "closed" => Some(IterationState::Closed),
            _ => None,
        }
    }
}

/// Where a task is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Backlog,
    Ready,
    InProgress,
    /// A person (or an outside event) must act; `waiting_on` says what, `who` optionally who.
    Waiting,
    /// Another task in the taskbag must finish first; `blocked_by` lists them (decision 0016).
    Blocked,
    Done,
    Dropped,
    /// A value this Allomos does not know, perhaps from a newer one (Task-A01-piano403): the task
    /// is shown with its value, kept in the task's `extra`, and this Allomos does not change it.
    #[serde(other)]
    Unknown,
}

impl State {
    pub const ALL: [State; 7] = [
        State::Backlog,
        State::Ready,
        State::InProgress,
        State::Waiting,
        State::Blocked,
        State::Done,
        State::Dropped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            State::Backlog => "backlog",
            State::Ready => "ready",
            State::InProgress => "in_progress",
            State::Waiting => "waiting",
            State::Blocked => "blocked",
            State::Done => "done",
            State::Dropped => "dropped",
            State::Unknown => "unknown",
        }
    }

    pub fn is_unknown(&self) -> bool {
        *self == State::Unknown
    }
}

/// One task. `description` comes from the task's `description.md`, not from `task.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub area: String,
    #[serde(skip_serializing_if = "State::is_unknown")]
    pub state: State,
    /// The highest step of the project's ladder this task has proven, if any.
    #[serde(default)]
    pub label: Option<String>,
    /// Who holds the task: a person or an agent session.
    #[serde(default)]
    pub holder: Option<String>,
    /// For a `waiting` task: what a person (or an outside event) must do, in plain words.
    #[serde(default)]
    pub waiting_on: Option<String>,
    /// For a `waiting` task: who must act, when it is not the approver (`alm wait --who`, decision
    /// 0016). Without it the app says "Waiting for you".
    #[serde(default)]
    pub who: Option<String>,
    /// For a `blocked` task: the tasks in this taskbag that must finish first (`alm block --by`,
    /// decision 0016). Empty for every other state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    #[serde(default)]
    pub next_dependency: Option<String>,
    /// The iteration the task is planned in, if any.
    #[serde(default)]
    pub iteration: Option<String>,
    /// A short note left on release for whoever picks the task up next.
    #[serde(default)]
    pub handoff_note: Option<String>,
    /// Why the task was finished without the human check: set by
    /// `alm done --without-human-check` (Task-A03-15, renamed in Task-A03-18). While a done task
    /// has it, the task waits for a person to check it ("To check", `alm human`). Marking it
    /// checked (`alm checked`, or the top label on a done task) clears it; its history keeps it.
    #[serde(default)]
    pub without_human_check: Option<String>,
    /// How a person checks the task, in Markdown (test folder, what to open, what to click, what
    /// they should see): `alm done --without-human-check … --steps <file.md>` (Task-A03-18).
    /// Cleared with the note when the task is checked.
    #[serde(default)]
    pub human_check_steps: Option<String>,
    /// `allomos/6`: the names of the project folders this task belongs to; empty, none in
    /// particular (decision 0019, Task-A01-dime494).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<String>,
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
    #[serde(default, skip_deserializing)]
    pub description: String,
}

impl Task {
    /// The state as the file writes it: for [`State::Unknown`], the value kept in `extra`.
    pub fn state_text(&self) -> String {
        match (self.state, self.extra.get("state").and_then(Value::as_str)) {
            (State::Unknown, Some(text)) => text.to_string(),
            (state, _) => state.as_str().to_string(),
        }
    }

    /// Finished without the human check and not checked since: "To check" (Task-A03-18).
    pub fn to_check(&self) -> bool {
        self.state == State::Done && self.without_human_check.is_some()
    }
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
