//! Glidebox project rules.
//!
//! This crate knows what a project, an area and a task are, and checks the JSON
//! that describes them. It never touches the disk: callers hand it file names and
//! file contents, and get back typed values plus a list of [`Issue`]s.

mod model;
mod problem;
mod validate;

pub use model::{Area, FORMAT, Project, State, Task, TaskBox, default_labels};
pub use problem::Problem;
pub use validate::{Issue, Loaded, parse_area, parse_project, parse_task, validate_task_box};
