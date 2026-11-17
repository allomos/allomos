//! The project's settings: `alm quota`, `alm handoff-rules`, `alm heartbeat-settings`, `alm audit-settings`, `alm task-language` and `alm labels`.

use std::path::Path;

use allomos_core::event::Kind;
use allomos_core::ops::{self};
use allomos_core::{Problem, Taskbag};
use allomos_store::{Actor, Lock, load, task_language, taskbag_of};
use serde_json::json;

use crate::{
    Cli, Command, Done, LabelsWhat, heartbeat, identity_lines, language_line, locate, lock_wait,
    retention_lines, rules_line, save_project, separation_lines, witness_lines,
};

/// Runs the commands this module holds; `execute` in `lib.rs` sends only those here.
pub(crate) fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::Quota {
            agent,
            account,
            left,
            resets,
            window,
        } => {
            let reports = match (agent, left) {
                (Some(agent), Some(left)) => {
                    let report = allomos_store::new_quota_report(
                        agent,
                        account.as_deref(),
                        window,
                        *left,
                        resets.as_deref(),
                    )?;
                    allomos_store::save_quota(report).map_err(|e| Problem::WriteFailed {
                        path: "quota.json".to_string(),
                        detail: e.to_string(),
                    })?
                }
                _ => allomos_store::quota_reports(),
            };
            let mut text = String::new();
            if agent.is_some() {
                text.push_str("Reported. Kept on this computer only: nothing to save or share.\n");
            }
            for r in &reports {
                text.push_str(&format!(
                    "{}{}  {}  {}% left{}  (reported {})\n",
                    r.agent,
                    r.account
                        .as_deref()
                        .map(|a| format!(" ({a})"))
                        .unwrap_or_default(),
                    r.window,
                    r.left,
                    r.resets
                        .as_deref()
                        .map(|t| format!(", resets {t}"))
                        .unwrap_or_default(),
                    r.at
                ));
            }
            if reports.is_empty() {
                text.push_str("No quota reported on this computer yet.\n");
            }
            Ok(Done::new(json!({ "reports": reports }), text))
        }
        Command::HandoffRules {
            quota,
            context,
            write_when,
            agent,
            clear,
        } => {
            let root = locate(cli, cwd)?;
            let changing = quota.is_some() || context.is_some() || write_when.is_some() || *clear;
            let (bag, issues) = if changing {
                let lock = Lock::acquire(&root, lock_wait())?;
                let (bag, issues) = taskbag_of(&root, lock.load())?;
                let change = ops::HandoffChange {
                    quota_gate: quota.as_deref(),
                    context_gate: context.as_deref(),
                    write_when: write_when.as_deref(),
                    agent: agent.as_deref(),
                    clear: *clear,
                };
                let project = ops::set_handoff_rules(&bag, &change)?;
                save_project(&lock, me, "handoff-rules", &bag.project, &project)?;
                (Taskbag { project, ..bag }, issues)
            } else {
                taskbag_of(&root, load(&root))?
            };
            let rules = bag.project.handoff.clone().unwrap_or_default();
            let applied = rules.applied(me.or(agent.as_deref()));
            let mut text = rules_line(&applied);
            for (name, g) in &rules.overrides {
                let part = |v: Option<u8>| {
                    v.map(|v| format!("{v}%"))
                        .unwrap_or_else(|| "project's".into())
                };
                text.push_str(&format!(
                    "  override for {name}: quota gate {}, context gate {}\n",
                    part(g.quota_gate),
                    part(g.context_gate)
                ));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "handoff_rules": rules, "applied": applied }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::HeartbeatSettings {
            interval,
            lost_after,
            push_work,
            clear,
        } => heartbeat::settings(
            cli,
            cwd,
            &allomos_core::heartbeat::SettingsChange {
                interval: interval.as_deref(),
                lost_after: lost_after.as_deref(),
                push_work: push_work.as_deref().map(|v| v == "on"),
                clear: *clear,
            },
        ),
        Command::AuditSettings {
            identity,
            reauthenticate,
            separation_of_duties,
            retention,
            legal_hold,
            witness,
            witness_service,
            witness_second,
            witness_when,
            confirm,
            reason,
        } => {
            let root = locate(cli, cwd)?;
            let witness_changing = witness.is_some()
                || witness_service.is_some()
                || witness_second.is_some()
                || witness_when.is_some();
            let changing = identity.is_some()
                || reauthenticate.is_some()
                || separation_of_duties.is_some()
                || retention.is_some()
                || legal_hold.is_some()
                || witness_changing;
            let (bag, issues, changed) = if changing {
                let lock = Lock::acquire(&root, lock_wait())?;
                let (bag, issues) = taskbag_of(&root, lock.load())?;
                let reason = reason.as_deref().or(cli.why.as_deref());
                let change = allomos_core::audit::IdentityChange {
                    level: identity.as_deref(),
                    reauthenticate: reauthenticate.as_deref().map(|v| v == "on"),
                    reason,
                };
                let (mut project, mut changes) = allomos_core::audit::set_identity(&bag, &change)?;
                if let Some(v) = separation_of_duties.as_deref() {
                    let (after, _) = allomos_core::duties::set(&project, v == "on", reason)?;
                    // One `audit` change from the first value to the last, one reason.
                    let before = bag.project.clone();
                    project = after;
                    changes = allomos_core::event::changes(Some(&before), &project, &[])
                        .into_iter()
                        .filter(|c| c.field == "audit")
                        .collect();
                    let why = reason.map(str::trim).filter(|r| !r.is_empty());
                    if let (false, Some(why)) = (changes.is_empty(), why) {
                        changes.push(allomos_core::event::reason_change(why));
                    }
                }
                if retention.is_some() || legal_hold.is_some() {
                    let years = retention
                        .as_deref()
                        .map(allomos_core::retention::parse)
                        .transpose()?;
                    let (after, _) = allomos_core::retention::set(
                        &project,
                        &allomos_core::retention::RetentionChange {
                            years,
                            legal_hold: legal_hold.as_deref().map(|v| v == "on"),
                            reason,
                        },
                    )?;
                    // One `audit` change from the first value to the last, one reason.
                    project = after;
                    changes = allomos_core::event::changes(Some(&bag.project), &project, &[])
                        .into_iter()
                        .filter(|c| c.field == "audit")
                        .collect();
                    let why = reason.map(str::trim).filter(|r| !r.is_empty());
                    if let (false, Some(why)) = (changes.is_empty(), why) {
                        changes.push(allomos_core::event::reason_change(why));
                    }
                }
                if witness_changing {
                    let (after, _) = allomos_core::witness::set(
                        &project,
                        &allomos_core::witness::WitnessChange {
                            on: witness.as_deref().map(|v| v == "on"),
                            service: witness_service.as_deref(),
                            second: witness_second.as_deref(),
                            when: witness_when.as_deref(),
                            confirmed: *confirm,
                            reason,
                        },
                    )?;
                    // One `audit` change from the first value to the last, one reason.
                    project = after;
                    changes = allomos_core::event::changes(Some(&bag.project), &project, &[])
                        .into_iter()
                        .filter(|c| c.field == "audit")
                        .collect();
                    let why = reason.map(str::trim).filter(|r| !r.is_empty());
                    if let (false, Some(why)) = (changes.is_empty(), why) {
                        changes.push(allomos_core::event::reason_change(why));
                    }
                }
                let changed = !changes.is_empty();
                let event = Actor::alm(me).event(
                    "audit-settings",
                    Kind::Project,
                    Some(&project.id),
                    changes,
                );
                lock.record(&event, |l| l.save_project(&project))?;
                (Taskbag { project, ..bag }, issues, Some(changed))
            } else {
                let (bag, issues) = taskbag_of(&root, load(&root))?;
                (bag, issues, None)
            };
            let id = allomos_core::audit::identity(&bag.project);
            let separated = allomos_core::duties::on(&bag.project);
            let mut text = String::new();
            if changed == Some(false) {
                text.push_str("Nothing changed: the audit settings already say that.\n");
            }
            text.push_str(&identity_lines(&id));
            text.push_str(&separation_lines(separated));
            let kept = allomos_core::retention::of(&bag.project);
            let today: String = allomos_store::events::now_utc().chars().take(10).collect();
            let past = kept
                .years
                .and_then(|y| allomos_store::redact::past_retention(&root, y, &today));
            text.push_str(&retention_lines(&kept, past.as_ref()));
            let witness_on = allomos_core::witness::of(&bag.project);
            text.push_str(&witness_lines(witness_on.as_ref()));
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "identity": id,
                    "separation_of_duties": separated,
                    "retention_years": kept.years,
                    "legal_hold": kept.legal_hold,
                    "past_retention": past,
                    "witness": witness_on,
                    "changed": changed
                }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::TaskLanguage { value: Some(value) } => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let project = ops::set_task_language(&bag, value)?;
            save_project(&lock, me, "task-language", &bag.project, &project)?;
            let system: Vec<String> = sys_locale::get_locales().collect();
            let now = task_language(&project, &system);
            let mut done = Done::new(
                json!({ "taskbag": root, "task_language": now }),
                format!(
                    "Task language set to {}. {}\n",
                    now.setting,
                    language_line(&now)
                ),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::TaskLanguage { value: None } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let system: Vec<String> = sys_locale::get_locales().collect();
            let now = task_language(&bag.project, &system);
            let mut done = Done::new(
                json!({ "taskbag": root, "task_language": now }),
                format!("{}\n", language_line(&now)),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Labels { what } => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let project = match what {
                None => bag.project.clone(),
                Some(LabelsWhat::Add { name, meaning }) => {
                    allomos_store::versions::upgraded_for_writing(&root)?;
                    allomos_core::labels::table_add(&bag, name, meaning.as_deref())?
                }
                Some(LabelsWhat::Remove { name }) => {
                    allomos_store::versions::upgraded_for_writing(&root)?;
                    allomos_core::labels::table_remove(&bag, name)?
                }
            };
            if what.is_some() {
                save_project(&lock, me, "labels", &bag.project, &project)?;
            }
            let mut text = String::from("Label table (built-in first):\n");
            let mut rows = Vec::new();
            for e in &project.labels {
                let n = bag.tasks.iter().filter(|t| t.has_label(&e.name)).count();
                text.push_str(&format!(
                    "  {}{}  ({n} task{}){}\n",
                    e.name,
                    if e.built_in { "  [built in]" } else { "" },
                    if n == 1 { "" } else { "s" },
                    e.meaning
                        .as_deref()
                        .map(|m| format!(": {m}"))
                        .unwrap_or_default()
                ));
                rows.push(json!({ "name": e.name, "meaning": e.meaning, "built_in": e.built_in, "tasks": n }));
            }
            let mut done = Done::new(json!({ "taskbag": root, "labels": rows }), text);
            done.issues = issues;
            Ok(done)
        }
        _ => unreachable!("lib.rs sends only this module's commands here"),
    }
}
