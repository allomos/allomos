//! The project's line limit for source files (Task-A03-dapper008, approver 2026-11-16):
//! `project.json`'s `file_size`. A source file over it may only get shorter, never longer, task by
//! task; each project checks that in its own tests (`alm guide file-size`). `alm` only keeps and
//! shows the number (`alm file-size-settings`) and repeats it in its reminders: it measures no
//! file.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, Project};
use crate::problem::Problem;

/// The line limit when the project sets none: 1000 lines (tests do not count).
pub const DEFAULT_LIMIT: u32 = 1000;

/// `project.json`'s `file_size`. A missing limit is the default; only a limit the project set
/// is written.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    /// The line limit for source files, tests not counted. Missing: 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_lines: Option<u32>,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The line limit as it applies: the project's, or 1000. A 0 (refused when written, reported by
/// validation) is read as the default.
pub fn limit(project: &Project) -> u32 {
    project
        .file_size
        .as_ref()
        .and_then(|s| s.limit_lines)
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_LIMIT)
}

/// A line limit as written for `alm file-size-settings --limit`: a whole number of lines, at
/// least 1.
pub fn parse_limit(text: &str) -> Result<u32, Problem> {
    let t = text.trim();
    t.parse::<u32>()
        .ok()
        .filter(|n| *n > 0 && t.chars().all(|c| c.is_ascii_digit()))
        .ok_or_else(|| Problem::FileSizeLimitInvalid {
            value: text.to_string(),
        })
}

/// The project with its line limit changed: `clear` first goes back to the default, then a given
/// `limit` is set. `file_size` goes away when nothing is left in it.
pub fn set_limit(project: &Project, limit: Option<&str>, clear: bool) -> Result<Project, Problem> {
    let limit = limit.map(parse_limit).transpose()?;
    let mut s = project.file_size.clone().unwrap_or_default();
    if clear {
        s.limit_lines = None;
    }
    if limit.is_some() {
        s.limit_lines = limit;
    }
    let mut project = project.clone();
    project.file_size = (s != Settings::default()).then_some(s);
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(json: &str) -> Project {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_limit_is_kept_shown_and_cleared_with_unknown_fields_kept() {
        let bare = project(r#"{"format":"allomos/9","id":"p","name":"P"}"#);
        assert_eq!(limit(&bare), DEFAULT_LIMIT);
        assert!(!serde_json::to_string(&bare).unwrap().contains("file_size"));

        let set = set_limit(&bare, Some("1200"), false).unwrap();
        assert_eq!(limit(&set), 1200);
        let json = serde_json::to_value(&set).unwrap();
        assert_eq!(json["file_size"]["limit_lines"], 1200);

        let back = set_limit(&set, None, true).unwrap();
        assert_eq!(back.file_size, None);
        assert_eq!(limit(&back), DEFAULT_LIMIT);

        // An unknown field under file_size stays when the limit is cleared.
        let newer = project(
            r#"{"format":"allomos/9","id":"p","name":"P","file_size":{"limit_lines":800,"later":true}}"#,
        );
        assert_eq!(limit(&newer), 800);
        let cleared = set_limit(&newer, None, true).unwrap();
        let json = serde_json::to_value(&cleared).unwrap();
        assert_eq!(json["file_size"], serde_json::json!({ "later": true }));

        // A 0 written by hand reads as the default.
        let zero =
            project(r#"{"format":"allomos/9","id":"p","name":"P","file_size":{"limit_lines":0}}"#);
        assert_eq!(limit(&zero), DEFAULT_LIMIT);
    }

    #[test]
    fn only_a_whole_number_of_lines_is_a_limit() {
        assert_eq!(parse_limit(" 1500 ").unwrap(), 1500);
        for bad in ["0", "-5", "1.5", "abc", "", "+10", "99999999999"] {
            assert!(
                matches!(parse_limit(bad), Err(Problem::FileSizeLimitInvalid { .. })),
                "{bad}"
            );
        }
    }
}
