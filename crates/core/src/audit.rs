//! A project's audit settings (decision 0021 § 6), kept in `project.json`'s `audit` so they travel
//! with the taskbag and everyone on the team works under the same rules.
//!
//! So far it holds the identity level (Task-A01-frolic587, decision 0021 § 2): where the identity
//! behind a pseudonym comes from, and whether approving and checking ask for a fresh sign-in.
//! Whatever the level, the history only ever holds a pseudonym ID; who is behind it is kept
//! outside the taskbag. Separation of duties (Task-A01-grotto036) sits beside it, with its rule in
//! [`crate::duties`]. Later tasks add the other switches (the outside witness, retention); until
//! then a newer Allomos's fields are kept as they are.

use serde::{Deserialize, Serialize};

use crate::model::{Extra, Project, Taskbag};
use crate::{Problem, event};

/// The default level: a fixed pseudonym ID per person per project, made by the app.
pub const PSEUDONYM: &str = "pseudonym";
/// The cloud project's account ID (decision 0006). Needs a cloud project.
pub const ACCOUNT: &str = "account";
/// The company's identity provider (OpenID Connect or SAML 2.0). Needs a cloud project.
pub const SSO: &str = "sso";
/// Every level, lowest first.
pub const LEVELS: [&str; 3] = [PSEUDONYM, ACCOUNT, SSO];

/// `project.json`'s `audit`. Every field is optional; a missing one takes its default.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Audit {
    /// One of [`LEVELS`]; missing means [`PSEUDONYM`]. A value this Allomos does not know (from
    /// a newer one) is kept and read as the pseudonym.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    /// "Sign in again to approve or check": only with [`ACCOUNT`] or [`SSO`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reauthenticate: bool,
    /// Separation of duties (Task-A01-grotto036): the same person or agent may not both do and
    /// check, or do and approve, a task. Off by default; see [`crate::duties`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub separation_of_duties: bool,
    /// Fields this Allomos does not know, kept as they are and written back (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: Extra,
}

/// The identity settings as they apply, defaults filled in, for `alm audit-settings` and the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identity {
    /// The level that applies (one of [`LEVELS`]).
    pub level: &'static str,
    /// As written in `project.json`, when it differs from `level` (a newer Allomos's value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<String>,
    pub reauthenticate: bool,
    /// Whether the sign-in levels can be chosen: only in a cloud project.
    pub sign_in_available: bool,
}

/// Whether `project` is a cloud project, where the sign-in levels can be chosen. This Allomos
/// has no cloud projects yet (decision 0006: they come with the cloud), so every project is
/// local-only and keeps the pseudonym.
pub fn cloud(_project: &Project) -> bool {
    false
}

fn known(level: &str) -> Option<&'static str> {
    LEVELS.iter().copied().find(|l| *l == level)
}

/// The identity settings that apply to `project`.
pub fn identity(project: &Project) -> Identity {
    let audit = project.audit.clone().unwrap_or_default();
    let written = audit.identity.clone();
    let level = written.as_deref().and_then(known).unwrap_or(PSEUDONYM);
    Identity {
        level,
        written: written.filter(|w| w != level),
        reauthenticate: audit.reauthenticate && level != PSEUDONYM,
        sign_in_available: cloud(project),
    }
}

/// A change to the identity settings: a new level, and or a new "sign in again" setting, and the
/// reason, recorded in the event (decision 0021 § 6).
#[derive(Debug, Default, Clone, Copy)]
pub struct IdentityChange<'a> {
    pub level: Option<&'a str>,
    pub reauthenticate: Option<bool>,
    pub reason: Option<&'a str>,
}

/// `project` with its identity settings changed, and the event's changes: the `audit` field's
/// from and to, plus the reason when one was given. Refused: an unknown level, a sign-in level in
/// a local-only project, signing in again with a pseudonym, and lowering the level or turning
/// signing in again off without a reason (decision 0021 § 6: turning a control off needs one).
pub fn set_identity(
    taskbag: &Taskbag,
    change: &IdentityChange,
) -> Result<(Project, Vec<event::Change>), Problem> {
    let before = &taskbag.project;
    let now = identity(before);
    let level = match change.level.map(str::trim) {
        Some(l) => known(&l.to_lowercase()).ok_or_else(|| Problem::IdentityLevelUnknown {
            value: l.to_string(),
        })?,
        None => now.level,
    };
    if level != PSEUDONYM && level != now.level && !cloud(before) {
        return Err(Problem::IdentityNeedsCloud {
            level: level.to_string(),
        });
    }
    let reauthenticate = change
        .reauthenticate
        .unwrap_or(now.reauthenticate && level != PSEUDONYM);
    if reauthenticate && level == PSEUDONYM {
        return Err(Problem::ReauthenticateNeedsSignIn);
    }
    let reason = change.reason.map(str::trim).filter(|r| !r.is_empty());
    let rank = |l: &str| LEVELS.iter().position(|x| *x == l).unwrap_or(0);
    if reason.is_none() {
        if rank(level) < rank(now.level) {
            return Err(Problem::AuditReasonMissing {
                setting: "identity".into(),
            });
        }
        if now.reauthenticate && !reauthenticate {
            return Err(Problem::AuditReasonMissing {
                setting: "reauthenticate".into(),
            });
        }
    }
    let mut audit = before.audit.clone().unwrap_or_default();
    // The default is not written down: a project that never chose keeps no `audit`.
    audit.identity = (level != PSEUDONYM || audit.identity.is_some()).then(|| level.to_string());
    if change.level.is_none() && audit.identity.as_deref() == Some(now.level) {
        // Keep a newer Allomos's value that was read as the pseudonym.
        audit.identity = before.audit.as_ref().and_then(|a| a.identity.clone());
    }
    audit.reauthenticate = reauthenticate;
    let empty = audit == Audit::default();
    let after = Project {
        audit: (!empty).then_some(audit),
        ..before.clone()
    };
    let mut changes: Vec<event::Change> = event::changes(Some(before), &after, &[])
        .into_iter()
        .filter(|c| c.field == "audit")
        .collect();
    if !changes.is_empty() {
        if let Some(why) = reason {
            changes.push(event::reason_change(why));
        }
    }
    Ok((after, changes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(audit: Option<Audit>) -> Taskbag {
        let project: Project = serde_json::from_value(serde_json::json!({
            "format": crate::FORMAT, "id": "p", "name": "P"
        }))
        .unwrap();
        Taskbag {
            project: Project { audit, ..project },
            areas: vec![],
            iterations: vec![],
            tasks: vec![],
        }
    }

    #[test]
    fn the_default_is_the_pseudonym_and_is_not_written() {
        let b = bag(None);
        let id = identity(&b.project);
        assert_eq!(id.level, PSEUDONYM);
        assert!(!id.reauthenticate && !id.sign_in_available);
        let (after, changes) = set_identity(
            &b,
            &IdentityChange {
                level: Some("Pseudonym"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(after.audit, None);
        assert!(changes.is_empty());
        assert!(!serde_json::to_string(&after).unwrap().contains("audit"));
    }

    #[test]
    fn a_local_project_cannot_choose_a_sign_in_level_or_sign_in_again() {
        let b = bag(None);
        for level in [ACCOUNT, SSO] {
            let e = set_identity(
                &b,
                &IdentityChange {
                    level: Some(level),
                    ..Default::default()
                },
            )
            .unwrap_err();
            assert_eq!(
                e,
                Problem::IdentityNeedsCloud {
                    level: level.into()
                }
            );
        }
        let e = set_identity(
            &b,
            &IdentityChange {
                reauthenticate: Some(true),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(e, Problem::ReauthenticateNeedsSignIn);
        let e = set_identity(
            &b,
            &IdentityChange {
                level: Some("passport"),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            e,
            Problem::IdentityLevelUnknown {
                value: "passport".into()
            }
        );
    }

    /// A taskbag a cloud-aware Allomos set to SSO with signing in again: lowering it needs a
    /// reason, which the event keeps beside the audit change.
    #[test]
    fn lowering_the_level_needs_a_reason_and_records_it() {
        let b = bag(Some(Audit {
            identity: Some(SSO.into()),
            reauthenticate: true,
            separation_of_duties: false,
            extra: Default::default(),
        }));
        assert_eq!(identity(&b.project).level, SSO);
        assert!(identity(&b.project).reauthenticate);
        let down = IdentityChange {
            level: Some(PSEUDONYM),
            reauthenticate: Some(false),
            reason: None,
        };
        assert_eq!(
            set_identity(&b, &down).unwrap_err(),
            Problem::AuditReasonMissing {
                setting: "identity".into()
            }
        );
        let (after, changes) = set_identity(
            &b,
            &IdentityChange {
                reason: Some("the team left the cloud"),
                ..down
            },
        )
        .unwrap();
        assert_eq!(identity(&after).level, PSEUDONYM);
        assert_eq!(
            after.audit.as_ref().unwrap().identity.as_deref(),
            Some(PSEUDONYM)
        );
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].field, "audit");
        assert_eq!(changes[1].field, event::REASON);
        // Turning signing in again off alone needs one too.
        let e = set_identity(
            &b,
            &IdentityChange {
                reauthenticate: Some(false),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            e,
            Problem::AuditReasonMissing {
                setting: "reauthenticate".into()
            }
        );
    }

    /// A newer Allomos's level is kept and read as the pseudonym; its other audit fields too.
    #[test]
    fn unknown_values_are_kept() {
        let mut extra = Extra::new();
        extra.insert("witness".into(), serde_json::json!({ "service": "x" }));
        let b = bag(Some(Audit {
            identity: Some("passkey".into()),
            reauthenticate: false,
            separation_of_duties: false,
            extra,
        }));
        let id = identity(&b.project);
        assert_eq!(id.level, PSEUDONYM);
        assert_eq!(id.written.as_deref(), Some("passkey"));
        let (after, changes) = set_identity(&b, &IdentityChange::default()).unwrap();
        assert_eq!(after, b.project);
        assert!(changes.is_empty());
    }
}
