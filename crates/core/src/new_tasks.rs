//! What an agent does with a task it has just added (Task-A00-wheat560, approver 2026-11-25):
//! `project.json`'s `new_tasks`. With `wait_for_approval` on, a new task stays in backlog until
//! the approver agrees: the agent tells the approver its ID and waits, instead of making it ready
//! itself. A goal run is the exception: submitting the goal approved its plan in advance. Off (the
//! default), the agent makes a task with no open questions ready itself. `alm` keeps and shows the
//! setting (`alm new-task-settings`) and words its next steps by it; it stops nothing.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, Project};

/// `project.json`'s `new_tasks`. Only a setting that is on is written.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    /// New tasks stay in backlog until the approver agrees. Missing: off.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wait_for_approval: bool,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Whether the project keeps new tasks in backlog until the approver agrees.
pub fn wait_for_approval(project: &Project) -> bool {
    project
        .new_tasks
        .as_ref()
        .is_some_and(|s| s.wait_for_approval)
}

/// The project with the setting turned on or off. `new_tasks` goes away when nothing is left in it.
pub fn set_wait_for_approval(project: &Project, on: bool) -> Project {
    let mut s = project.new_tasks.clone().unwrap_or_default();
    s.wait_for_approval = on;
    let mut project = project.clone();
    project.new_tasks = (s != Settings::default()).then_some(s);
    project
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(json: &str) -> Project {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_setting_is_off_by_default_kept_when_on_and_gone_when_off() {
        let bare = project(r#"{"format":"allomos/9","id":"p","name":"P"}"#);
        assert!(!wait_for_approval(&bare));
        assert!(!serde_json::to_string(&bare).unwrap().contains("new_tasks"));

        let on = set_wait_for_approval(&bare, true);
        assert!(wait_for_approval(&on));
        let json = serde_json::to_value(&on).unwrap();
        assert_eq!(
            json["new_tasks"],
            serde_json::json!({ "wait_for_approval": true })
        );

        let off = set_wait_for_approval(&on, false);
        assert_eq!(off.new_tasks, None);
        assert_eq!(off, bare);

        // An unknown field under new_tasks stays when the setting is turned off.
        let newer = project(
            r#"{"format":"allomos/9","id":"p","name":"P","new_tasks":{"wait_for_approval":true,"later":1}}"#,
        );
        assert!(wait_for_approval(&newer));
        let json = serde_json::to_value(set_wait_for_approval(&newer, false)).unwrap();
        assert_eq!(json["new_tasks"], serde_json::json!({ "later": 1 }));
    }
}
