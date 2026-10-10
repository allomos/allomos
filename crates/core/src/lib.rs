//! Allomos project rules.
//!
//! This crate knows what a project, an area, an iteration and a task are, and checks the JSON
//! that describes them. It never touches the disk: callers hand it file names and
//! file contents, and get back typed values plus a list of [`Issue`]s.

mod model;
pub mod ops;
mod problem;
mod validate;

pub use model::{
    Area, FORMAT, Iteration, LEGACY_FORMAT, Project, State, TASK_LANGUAGE_UI, Task, Taskbag,
    default_labels, valid_language,
};
pub use problem::Problem;
pub use validate::{
    Issue, Loaded, parse_area, parse_iteration, parse_project, parse_task, valid_date,
    validate_taskbag,
};
