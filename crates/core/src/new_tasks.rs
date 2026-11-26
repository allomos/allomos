//! What an agent does with a task it has just added (Task-A00-wheat560, approver 2026-11-25):
//! `project.json`'s `new_tasks`. With `wait_for_approval` on, a new task stays in backlog until
//! the approver agrees: the agent tells the approver its ID and waits, instead of making it ready
//! itself. This is the default. A goal run is the exception: submitting the goal approved its plan
//! in advance. When off, the agent makes a task with no open questions ready itself.
//! `alm` keeps and shows the setting (`alm new-task-settings`) and words its next steps by it;
//! it stops nothing.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, Project};

/// `project.json`'s `new_tasks`. A saved choice is written explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// New tasks stay in backlog until the approver agrees. Missing: on.
    #[serde(default = "approval_default")]
    pub wait_for_approval: bool,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

fn approval_default() -> bool {
    true
}

/// Whether the project keeps new tasks in backlog until the approver agrees.
pub fn wait_for_approval(project: &Project) -> bool {
    project
        .new_tasks
        .as_ref()
        .map_or_else(approval_default, |s| s.wait_for_approval)
}

/// The project with the setting turned on or off.
pub fn set_wait_for_approval(project: &Project, on: bool) -> Project {
    let mut updated = project.clone();
    if wait_for_approval(project) != on {
        let extra = updated.new_tasks.take().map(|settings| settings.extra);
        updated.new_tasks = Some(Settings {
            wait_for_approval: on,
            extra: extra.unwrap_or_default(),
        });
    }
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(json: &str) -> Project {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_setting_is_on_by_default_and_explicit_choices_persist() {
        let bare = project(r#"{"format":"allomos/9","id":"p","name":"P"}"#);
        assert!(wait_for_approval(&bare));
        assert_eq!(set_wait_for_approval(&bare, true), bare);

        let off = set_wait_for_approval(&bare, false);
        let json = serde_json::to_value(&off).unwrap();
        assert_eq!(
            json["new_tasks"],
            serde_json::json!({ "wait_for_approval": false })
        );
        let reloaded: Project = serde_json::from_value(json).unwrap();
        assert!(!wait_for_approval(&reloaded));

        let on = set_wait_for_approval(&reloaded, true);
        assert!(wait_for_approval(&on));
        let json = serde_json::to_value(&on).unwrap();
        assert_eq!(
            json["new_tasks"],
            serde_json::json!({ "wait_for_approval": true })
        );

        // An unknown field under new_tasks stays when the setting is turned off.
        let newer =
            project(r#"{"format":"allomos/9","id":"p","name":"P","new_tasks":{"later":1}}"#);
        assert!(wait_for_approval(&newer));
        let json = serde_json::to_value(set_wait_for_approval(&newer, false)).unwrap();
        assert_eq!(
            json["new_tasks"],
            serde_json::json!({ "wait_for_approval": false, "later": 1 })
        );
    }
}
