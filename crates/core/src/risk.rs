//! Risk management settings (decision 0027 § 4, 5 and 7, Task-A01-beetle749): what a project adds
//! to the fixed rules for what counts as high risk, and its two limits.
//!
//! The fixed rules (H1 to H7) are not kept in the taskbag: a project cannot remove them. Its own
//! rules are sentences written for the agent that classifies a task (`alm classify`), one each;
//! Allomos never matches them against anything (decision 0027 § 3). The two limits are off by
//! default; this module keeps them, and `alm` applies them (Task-A01-stable046).

use serde::{Deserialize, Serialize};

use crate::model::{Extra, Project};

/// The fixed rules for high risk (decision 0027 § 1), by their short names. Their wording is the
/// reader's: the app words them in its own language.
pub const FIXED_HIGH: [&str; 7] = ["H1", "H2", "H3", "H4", "H5", "H6", "H7"];

/// `project.json`'s `risk` (decision 0027 § 7). A taskbag without it has no rules of its own and
/// both limits off; nothing is written while that is so.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Settings {
    /// The project's own rules, in the order the project put them: one sentence each, on one line.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<String>,
    /// Limit 1 (decision 0027 § 5): a high-risk task cannot be finished without the human check
    /// (`alm done --without-human-check`). Missing: off, so only `true` is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_needs_human_check: Option<bool>,
    /// Limit 2 (decision 0027 § 5): in a quiet run, an agent does not merge a high-risk task
    /// itself; it stops for the approver. Missing: off, so only `true` is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_needs_approver_merge: Option<bool>,
    /// Fields this Allomos does not know, kept as they are.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The settings as they apply, defaults filled in.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Applied {
    pub rules: Vec<String>,
    pub high_needs_human_check: bool,
    pub high_needs_approver_merge: bool,
}

/// The project's risk settings as they apply.
pub fn applied(project: &Project) -> Applied {
    let s = project.risk.clone().unwrap_or_default();
    Applied {
        rules: clean_rules(&s.rules),
        high_needs_human_check: s.high_needs_human_check.unwrap_or(false),
        high_needs_approver_merge: s.high_needs_approver_merge.unwrap_or(false),
    }
}

/// A change to the risk settings: `rules` replaces the whole list of the project's own rules, each
/// limit given is turned on or off.
#[derive(Debug, Default)]
pub struct SettingsChange<'a> {
    pub rules: Option<&'a [String]>,
    pub high_needs_human_check: Option<bool>,
    pub high_needs_approver_merge: Option<bool>,
}

/// One rule as it is kept: on one line, without spaces around it.
fn clean_rule(rule: &str) -> String {
    rule.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The rules as they are kept: each on one line, no empty one, none twice (the first stays).
fn clean_rules(rules: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for rule in rules.iter().map(|r| clean_rule(r)) {
        if !rule.is_empty() && !kept.contains(&rule) {
            kept.push(rule);
        }
    }
    kept
}

/// The project with its risk settings changed; `risk` goes away when the project has no rules of
/// its own, both limits are off and nothing unknown is kept.
pub fn set_settings(project: &Project, change: &SettingsChange) -> Project {
    let mut s = project.risk.clone().unwrap_or_default();
    if let Some(rules) = change.rules {
        s.rules = clean_rules(rules);
    }
    if let Some(on) = change.high_needs_human_check {
        s.high_needs_human_check = on.then_some(true);
    }
    if let Some(on) = change.high_needs_approver_merge {
        s.high_needs_approver_merge = on.then_some(true);
    }
    let mut project = project.clone();
    project.risk = (s != Settings::default()).then_some(s);
    project
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(json: &str) -> Project {
        serde_json::from_str(json).unwrap()
    }

    const BARE: &str = r#"{"format":"allomos/9","id":"p","name":"P"}"#;

    /// Decision 0027 § 7: a taskbag without `risk` has no rules of its own and both limits off.
    #[test]
    fn a_taskbag_without_the_field_has_no_rules_and_both_limits_off() {
        let p = project(BARE);
        assert_eq!(p.risk, None);
        assert_eq!(applied(&p), Applied::default());
        assert!(!serde_json::to_string(&p).unwrap().contains("risk"));
    }

    #[test]
    fn rules_are_added_changed_and_removed_and_kept_one_line_each() {
        let p = project(BARE);
        let rules = vec![
            "  Changing the price list\n is high risk. ".to_string(),
            "".to_string(),
            "Touching the importer is high risk.".to_string(),
            "Changing the price list is high risk.".to_string(),
        ];
        let p = set_settings(
            &p,
            &SettingsChange {
                rules: Some(&rules),
                ..Default::default()
            },
        );
        assert_eq!(
            applied(&p).rules,
            [
                "Changing the price list is high risk.",
                "Touching the importer is high risk."
            ]
        );
        // Changing one and removing the other: the list given replaces the list kept.
        let p = set_settings(
            &p,
            &SettingsChange {
                rules: Some(&["Changing any price is high risk.".to_string()]),
                ..Default::default()
            },
        );
        assert_eq!(applied(&p).rules, ["Changing any price is high risk."]);
        // A change of a limit leaves the rules alone.
        let p = set_settings(
            &p,
            &SettingsChange {
                high_needs_human_check: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(applied(&p).rules.len(), 1);
        // No rule left and the limit off again: the field leaves project.json.
        let p = set_settings(
            &p,
            &SettingsChange {
                rules: Some(&[]),
                high_needs_human_check: Some(false),
                ..Default::default()
            },
        );
        assert_eq!(p.risk, None);
    }

    #[test]
    fn each_limit_is_off_by_default_and_only_written_when_on() {
        let p = project(BARE);
        let on = set_settings(
            &p,
            &SettingsChange {
                high_needs_approver_merge: Some(true),
                ..Default::default()
            },
        );
        let a = applied(&on);
        assert!(a.high_needs_approver_merge && !a.high_needs_human_check);
        let json = serde_json::to_value(&on).unwrap();
        assert_eq!(
            json["risk"],
            serde_json::json!({"high_needs_approver_merge": true})
        );
        let off = set_settings(
            &on,
            &SettingsChange {
                high_needs_approver_merge: Some(false),
                ..Default::default()
            },
        );
        assert_eq!(off, p);
    }

    /// A field a newer Allomos wrote inside `risk` is kept through a change.
    #[test]
    fn an_unknown_field_inside_is_kept() {
        let p = project(
            r#"{"format":"allomos/9","id":"p","name":"P","risk":{"rules":["a"],"later":1}}"#,
        );
        let p = set_settings(
            &p,
            &SettingsChange {
                rules: Some(&[]),
                ..Default::default()
            },
        );
        assert_eq!(
            serde_json::to_value(&p).unwrap()["risk"],
            serde_json::json!({"later": 1})
        );
    }
}
