//! Allomos project rules.
//!
//! This crate knows what a project, an area, an iteration, a task and an event are, and checks the JSON
//! that describes them. It never touches the disk: callers hand it file names and
//! file contents, and get back typed values plus a list of [`Issue`]s.

pub mod decision;
pub mod event;
mod model;
pub mod ops;
mod problem;
mod validate;

pub use model::{
    AppliedRules, Area, DEFAULT_CONTEXT_GATE, DEFAULT_QUOTA_GATE, DEFAULT_WRITE_WHEN, FORMAT,
    FORMAT_VERSION, Gates, HandoffRules, Iteration, Project, State, TASK_LANGUAGE_UI, Task,
    Taskbag, default_labels, format_version, valid_language,
};
pub use problem::Problem;
pub use validate::{
    Issue, Loaded, parse_area, parse_event, parse_iteration, parse_project, parse_task, valid_date,
    validate_taskbag,
};
