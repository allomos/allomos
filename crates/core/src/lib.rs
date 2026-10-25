//! Allomos project rules.
//!
//! This crate knows what a project, an area, an iteration, a task and an event are, and checks the JSON
//! that describes them. It never touches the disk: callers hand it file names and
//! file contents, and get back typed values plus a list of [`Issue`]s.

pub mod branch;
pub mod card;
pub mod decision;
pub mod event;
pub mod ids;
mod model;
pub mod ops;
mod problem;
pub mod request;
mod validate;

pub use model::{
    AppliedRules, Area, DEFAULT_CONTEXT_GATE, DEFAULT_QUOTA_GATE, DEFAULT_WRITE_WHEN, Extra,
    FORMAT, FORMAT_VERSION, Gates, HandoffRules, Iteration, IterationState, Project, State,
    TASK_LANGUAGE_UI, Task, Taskbag, default_labels, format_version, valid_language,
};
pub use problem::Problem;
pub use validate::{
    Issue, Loaded, event_notes, parse_area, parse_event, parse_iteration, parse_project,
    parse_task, unknown_notes, valid_date, validate_taskbag,
};
