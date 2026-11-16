//! The project's risk management settings in the app (decision 0027 § 6, Task-A01-beetle749): the
//! Risk management section of the taskbag's settings shows the fixed rules for high risk (H1 to
//! H7, which a project cannot change), and shows and changes `project.json`'s `risk`: the rules
//! the project adds and its two limits, both off by default.

use std::path::PathBuf;

use allomos_core::Problem;
use allomos_core::risk::{self, SettingsChange};
use serde::Serialize;

/// The settings as they apply, with the fixed rules' short names so the app keeps no list of its
/// own.
#[derive(Debug, Serialize)]
pub(crate) struct RiskSettings {
    /// The fixed rules for high risk, by short name (`H1`…); the app words them.
    fixed: Vec<&'static str>,
    /// The project's own rules, one sentence each.
    rules: Vec<String>,
    high_needs_human_check: bool,
    high_needs_approver_merge: bool,
}

fn of_project(project: &allomos_core::Project) -> RiskSettings {
    let applied = risk::applied(project);
    RiskSettings {
        fixed: risk::FIXED_HIGH.to_vec(),
        rules: applied.rules,
        high_needs_human_check: applied.high_needs_human_check,
        high_needs_approver_merge: applied.high_needs_approver_merge,
    }
}

#[tauri::command(async)]
pub(crate) fn risk_settings(root: PathBuf) -> Result<RiskSettings, Problem> {
    let loaded = allomos_store::load(&root);
    let Some(bag) = loaded.taskbag else {
        return Err(loaded.issues.first().map(|i| i.problem.clone()).unwrap_or(
            Problem::NotATaskbag {
                path: root.display().to_string(),
            },
        ));
    };
    Ok(of_project(&bag.project))
}

/// Change the risk settings: `rules` replaces the project's own rules (an empty sentence and a
/// repeated one are left out), each limit given is turned on or off. Returns the settings as
/// saved; nothing is written when they already say that.
#[tauri::command(async)]
pub(crate) fn set_risk_settings(
    root: PathBuf,
    rules: Option<Vec<String>>,
    high_needs_human_check: Option<bool>,
    high_needs_approver_merge: Option<bool>,
) -> Result<RiskSettings, Problem> {
    let lock = allomos_store::Lock::acquire(&root, std::time::Duration::from_secs(5))?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let change = SettingsChange {
        rules: rules.as_deref(),
        high_needs_human_check,
        high_needs_approver_merge,
    };
    let project = risk::set_settings(&bag.project, &change);
    if project != bag.project {
        let event = allomos_store::Actor::APP.event(
            "risk-settings",
            allomos_core::event::Kind::Project,
            Some(&project.id),
            allomos_core::event::changes(Some(&bag.project), &project, &[]),
        );
        lock.record(&event, |l| l.save_project(&project))?;
    }
    Ok(of_project(&project))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_and_limits_are_shown_changed_and_saved_in_project_json() {
        let dir = std::env::temp_dir().join(format!("allomos-desktop-risk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let file = || std::fs::read_to_string(root.join("project.json")).unwrap();

        // Nothing written: the seven fixed rules, no rule of the project's own, both limits off.
        let s = risk_settings(root.clone()).unwrap();
        assert_eq!(s.fixed, ["H1", "H2", "H3", "H4", "H5", "H6", "H7"]);
        assert!(s.rules.is_empty() && !s.high_needs_human_check && !s.high_needs_approver_merge);
        assert!(!file().contains("\"risk\""));

        // Adding rules: saved in project.json, an empty sentence left out.
        let rules = vec![
            "Changing prices is high risk.".to_string(),
            "  ".to_string(),
        ];
        let s = set_risk_settings(root.clone(), Some(rules), None, None).unwrap();
        assert_eq!(s.rules, ["Changing prices is high risk."]);
        assert!(file().contains("Changing prices is high risk."));

        // A limit turned on leaves the rules; it is read back from the file.
        let s = set_risk_settings(root.clone(), None, Some(true), None).unwrap();
        assert!(s.high_needs_human_check && !s.high_needs_approver_merge);
        let s = risk_settings(root.clone()).unwrap();
        assert_eq!(s.rules.len(), 1);
        assert!(s.high_needs_human_check);

        // Each change is an event; the same settings again write nothing.
        let events = || std::fs::read_dir(root.join("events")).unwrap().count();
        let before = events();
        set_risk_settings(root.clone(), None, Some(true), None).unwrap();
        assert_eq!(events(), before);
        set_risk_settings(root.clone(), None, None, Some(true)).unwrap();
        assert_eq!(events(), before + 1);

        // All removed and off: `risk` leaves project.json.
        set_risk_settings(root.clone(), Some(vec![]), Some(false), Some(false)).unwrap();
        assert!(!file().contains("\"risk\""));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
