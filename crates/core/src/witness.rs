//! The outside witness's setting (Task-A01-wood982, decision 0021 § 3.3 and § 6 row 4): which
//! RFC 3161 time-stamp service countersigns each checkpoint's fingerprint, kept in
//! `project.json`'s `audit.witness` so everyone on the team sends to the same service.
//!
//! Off by default. Only a checkpoint's fingerprint (a SHA-256 hash) is ever sent, never the
//! taskbag's content. Turning it on is the person's choice, confirmed (in `alm` with `--confirm`,
//! in the app with a confirmation); turning it off needs a reason, as every audit control does.
//! Sending and the receipts themselves are in the store crate (`checkpoint`, `tsa`).

use serde::{Deserialize, Serialize};

use crate::model::Project;
use crate::{Problem, event};

/// `audit.witness` in `project.json`: present means on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Witness {
    /// The service's address (`https://…` or `http://…`), a preset's or a custom one.
    pub service: String,
    /// An optional second service, so one service closing does not end the chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second: Option<String>,
    /// How often checkpoints are made by themselves (Task-A01-bake791): `daily`, `approvals` or
    /// `asked`; see [`When`]. Kept as written, so a value from a newer Allomos survives; a value
    /// this one does not know, or none, counts as `asked` (nothing is made by itself).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Fields this Allomos does not know, kept as they are (Task-A01-piano403).
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// How often the outside witness's checkpoints are made by themselves (Task-A01-bake791).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum When {
    /// Once a day: by the app while it is open, and by any `alm` command that changes the
    /// taskbag, when the last checkpoint is a day old or more.
    Daily,
    /// After each approval and each human check (`alm approve`, `alm checked`, the app's same).
    Approvals,
    /// Only when asked: `alm checkpoint`, **Make a checkpoint now**, and on upgrading.
    Asked,
}

impl When {
    pub const ALL: [When; 3] = [When::Daily, When::Approvals, When::Asked];
    /// What turning the witness on chooses when nothing is said (the task's outcome).
    pub const FIRST: When = When::Daily;

    pub fn id(self) -> &'static str {
        match self {
            When::Daily => "daily",
            When::Approvals => "approvals",
            When::Asked => "asked",
        }
    }

    /// `daily`, `approvals` or `asked` (any case). Refused: anything else (`witness_when_invalid`).
    pub fn parse(value: &str) -> Result<When, Problem> {
        let v = value.trim();
        When::ALL
            .into_iter()
            .find(|w| w.id().eq_ignore_ascii_case(v))
            .ok_or_else(|| Problem::WitnessWhenInvalid {
                value: v.to_string(),
            })
    }
}

impl Witness {
    /// How often checkpoints are made by themselves; `asked` when unset or unknown.
    pub fn when(&self) -> When {
        self.when
            .as_deref()
            .and_then(|w| When::parse(w).ok())
            .unwrap_or(When::Asked)
    }

    /// The services each checkpoint is sent to, first one first.
    pub fn services(&self) -> Vec<String> {
        let mut out = vec![self.service.clone()];
        out.extend(self.second.clone().filter(|s| *s != self.service));
        out
    }
}

/// One of the services offered by name (decision 0021 § 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Preset {
    /// What `alm` and the app call it (`--witness-service freetsa`).
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    /// The default, approved for this use (approver, 2026-11-08). The others are only listed:
    /// none of them says whether it may be used this way.
    pub default: bool,
}

/// The presets, the default first. A company's own (paid, qualified) service is a custom address.
pub const PRESETS: [Preset; 4] = [
    Preset {
        id: "freetsa",
        name: "FreeTSA.org",
        url: "https://freetsa.org/tsr",
        default: true,
    },
    Preset {
        id: "digicert",
        name: "DigiCert",
        url: "http://timestamp.digicert.com",
        default: false,
    },
    Preset {
        id: "sectigo",
        name: "Sectigo",
        url: "http://timestamp.sectigo.com",
        default: false,
    },
    Preset {
        id: "globalsign",
        name: "GlobalSign",
        url: "http://timestamp.globalsign.com/tsa/r45standard",
        default: false,
    },
];

/// The setting's name in events and problems.
pub const FIELD: &str = "witness";

/// The outside witness, when it is on.
pub fn of(project: &Project) -> Option<Witness> {
    project.audit.as_ref().and_then(|a| a.witness.clone())
}

/// A preset's ID or name, or an `http(s)://` address, as the service's address. Refused: anything
/// else (`witness_service_invalid`).
pub fn service_url(value: &str) -> Result<String, Problem> {
    let v = value.trim();
    if let Some(p) = PRESETS
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(v) || p.name.eq_ignore_ascii_case(v))
    {
        return Ok(p.url.to_string());
    }
    let lower = v.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    match rest {
        Some(host) if !host.is_empty() && !v.chars().any(char::is_whitespace) => Ok(v.to_string()),
        _ => Err(Problem::WitnessServiceInvalid {
            value: v.to_string(),
        }),
    }
}

/// A change to the outside witness: on (with its services) or off, and the reason.
#[derive(Debug, Default, Clone)]
pub struct WitnessChange<'a> {
    /// `Some(true)` on, `Some(false)` off, `None` keep (and change only the services).
    pub on: Option<bool>,
    /// A preset or an address; missing when turning on: the default preset.
    pub service: Option<&'a str>,
    /// A preset or an address; `Some("")` or `Some("none")` removes the second service.
    pub second: Option<&'a str>,
    /// `daily`, `approvals` or `asked`; missing when turning on: `daily` (Task-A01-bake791).
    pub when: Option<&'a str>,
    /// The person said yes to sending fingerprints out (needed to turn it on).
    pub confirmed: bool,
    pub reason: Option<&'a str>,
}

/// `project` with the outside witness changed, and the event's changes: the `audit` field's from
/// and to, plus the reason. Refused: turning it on without the person's confirmation, an invalid
/// address, turning it off without a reason, changing the services while it is off.
pub fn set(
    project: &Project,
    change: &WitnessChange,
) -> Result<(Project, Vec<event::Change>), Problem> {
    let reason = change.reason.map(str::trim).filter(|r| !r.is_empty());
    let now = of(project);
    let turn_on = change.on.unwrap_or(now.is_some());
    let next = if turn_on {
        if now.is_none() && !change.confirmed {
            return Err(Problem::WitnessNotConfirmed);
        }
        let default = PRESETS[0].url.to_string();
        let service = match change.service {
            Some(s) => service_url(s)?,
            None => now.as_ref().map_or(default, |w| w.service.clone()),
        };
        let second = match change.second.map(str::trim) {
            Some("") => None,
            Some(s) if s.eq_ignore_ascii_case("none") => None,
            Some(s) => Some(service_url(s)?),
            None => now.as_ref().and_then(|w| w.second.clone()),
        }
        .filter(|s| *s != service);
        let when = match change.when {
            Some(w) => Some(When::parse(w)?.id().to_string()),
            None => match &now {
                Some(w) => w.when.clone(),
                None => Some(When::FIRST.id().to_string()),
            },
        };
        Some(Witness {
            service,
            second,
            when,
            extra: now.as_ref().map(|w| w.extra.clone()).unwrap_or_default(),
        })
    } else {
        if change.service.is_some() || change.second.is_some() || change.when.is_some() {
            return Err(Problem::WitnessOff);
        }
        None
    };
    if next == now {
        return Ok((project.clone(), Vec::new()));
    }
    if next.is_none() && reason.is_none() {
        return Err(Problem::AuditReasonMissing {
            setting: FIELD.into(),
        });
    }
    let mut audit = project.audit.clone().unwrap_or_default();
    audit.witness = next;
    let empty = audit == crate::audit::Audit::default();
    let after = Project {
        audit: (!empty).then_some(audit),
        ..project.clone()
    };
    let mut changes: Vec<event::Change> = event::changes(Some(project), &after, &[])
        .into_iter()
        .filter(|c| c.field == "audit")
        .collect();
    if let Some(why) = reason {
        changes.push(event::reason_change(why));
    }
    Ok((after, changes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn project() -> Project {
        serde_json::from_value(json!({ "format": crate::FORMAT, "id": "p", "name": "P" })).unwrap()
    }

    fn on(p: &Project, service: Option<&str>) -> Result<(Project, Vec<event::Change>), Problem> {
        set(
            p,
            &WitnessChange {
                on: Some(true),
                service,
                confirmed: true,
                ..Default::default()
            },
        )
    }

    #[test]
    fn off_by_default_and_on_only_when_confirmed() {
        let p = project();
        assert_eq!(of(&p), None);
        let refused = set(
            &p,
            &WitnessChange {
                on: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(refused, Err(Problem::WitnessNotConfirmed));
        let (p, changes) = on(&p, None).unwrap();
        let w = of(&p).unwrap();
        assert_eq!(w.service, "https://freetsa.org/tsr");
        assert_eq!(w.services(), vec!["https://freetsa.org/tsr".to_string()]);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].field, "audit");
        let text = serde_json::to_string(&p).unwrap();
        assert!(text.contains(r#""witness":{"service":"https://freetsa.org/tsr","when":"daily"}"#));
        assert_eq!(w.when(), When::Daily);
    }

    #[test]
    fn how_often_is_daily_first_and_kept_as_written() {
        let (p, _) = on(&project(), None).unwrap();
        let (p, changes) = set(
            &p,
            &WitnessChange {
                when: Some("Approvals"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(of(&p).unwrap().when(), When::Approvals);
        assert_eq!(
            set(
                &p,
                &WitnessChange {
                    when: Some("hourly"),
                    ..Default::default()
                }
            ),
            Err(Problem::WitnessWhenInvalid {
                value: "hourly".into()
            })
        );
        // Changing only the service keeps how often.
        let (p, _) = set(
            &p,
            &WitnessChange {
                service: Some("sectigo"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(of(&p).unwrap().when(), When::Approvals);
        // None, or a value from a newer Allomos: only when asked, and the value is kept.
        let w: Witness =
            serde_json::from_value(json!({ "service": "https://x.example", "when": "hourly" }))
                .unwrap();
        assert_eq!(w.when(), When::Asked);
        assert!(
            serde_json::to_string(&w)
                .unwrap()
                .contains(r#""when":"hourly""#)
        );
        let w: Witness = serde_json::from_value(json!({ "service": "https://x.example" })).unwrap();
        assert_eq!(w.when(), When::Asked);
    }

    #[test]
    fn presets_and_custom_addresses() {
        assert_eq!(
            service_url("DigiCert").unwrap(),
            "http://timestamp.digicert.com"
        );
        assert_eq!(
            service_url(" https://tsa.example.com/tsr ").unwrap(),
            "https://tsa.example.com/tsr"
        );
        for bad in ["", "ftp://x", "https://", "tsa.example.com", "https://a b"] {
            assert!(
                matches!(service_url(bad), Err(Problem::WitnessServiceInvalid { .. })),
                "{bad}"
            );
        }
    }

    #[test]
    fn off_needs_a_reason_and_a_second_service_can_be_added_and_removed() {
        let (p, _) = on(&project(), Some("freetsa")).unwrap();
        let (p, _) = set(
            &p,
            &WitnessChange {
                second: Some("sectigo"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            of(&p).unwrap().services(),
            vec![
                "https://freetsa.org/tsr".to_string(),
                "http://timestamp.sectigo.com".to_string()
            ]
        );
        let (p, _) = set(
            &p,
            &WitnessChange {
                second: Some("none"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(of(&p).unwrap().second, None);
        let off = WitnessChange {
            on: Some(false),
            ..Default::default()
        };
        assert_eq!(
            set(&p, &off),
            Err(Problem::AuditReasonMissing {
                setting: "witness".into()
            })
        );
        let (p, changes) = set(
            &p,
            &WitnessChange {
                reason: Some("trial over"),
                ..off
            },
        )
        .unwrap();
        assert_eq!(of(&p), None);
        assert_eq!(p.audit, None);
        assert_eq!(changes.len(), 2);
        // Already off: nothing changes, no reason needed.
        assert_eq!(set(&p, &off).unwrap().1, Vec::new());
    }
}
