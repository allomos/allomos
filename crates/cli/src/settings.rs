//! The project's settings: `alm quota`, `alm handoff-rules`, `alm heartbeat-settings`, `alm
//! file-size-settings`, `alm new-task-settings`, `alm audit-settings`, `alm task-language` and `alm labels`.

use std::path::Path;

use allomos_core::event::Kind;
use allomos_core::ops;
use allomos_core::{Problem, Taskbag};
use allomos_store::{Actor, Lock, load, task_language, taskbag_of};
use serde_json::json;

use crate::{
    Cli, Command, Done, LabelsWhat, heartbeat, locate, lock_wait, rules_line, save_project,
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
        Command::FileSizeSettings { limit, clear } => {
            file_size_settings(cli, cwd, limit.as_deref(), *clear)
        }
        Command::NewTaskSettings { wait_for_approval } => {
            new_task_settings(cli, cwd, wait_for_approval.as_deref().map(|v| v == "on"))
        }
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

/// The identity settings in plain words (`alm audit-settings`).
pub(crate) fn identity_lines(id: &allomos_core::audit::Identity) -> String {
    let level = match id.level {
        allomos_core::audit::ACCOUNT => "Allomos account",
        allomos_core::audit::SSO => "Company sign-in (SSO)",
        _ => "Pseudonym",
    };
    let mut text = format!("Identity: {level}");
    if let Some(w) = &id.written {
        text.push_str(&format!(
            " (project.json says \"{w}\", which this Allomos does not know)"
        ));
    }
    text.push_str(&format!(
        ".\nSign in again to approve or check: {}.\n",
        if id.reauthenticate { "on" } else { "off" }
    ));
    text.push_str(
        "The history only ever holds a pseudonym ID; who is behind each one is kept on this computer (local project) or in the cloud (cloud project), never in the taskbag.\n",
    );
    if !id.sign_in_available {
        text.push_str(
            "Allomos account and Company sign-in (SSO) need a cloud project; this project is local-only.\n",
        );
    }
    text
}

/// Separation of duties in plain words (`alm audit-settings`, Task-A01-grotto036).
pub(crate) fn separation_lines(on: bool) -> String {
    if on {
        "Separation of duties: on. The same person or agent may not both do and check (or do and approve) a task; alm refuses it and says why. A project with one person cannot finish its checks while this is on.\n".into()
    } else {
        "Separation of duties: off. Anyone may check or approve a task, including who did it.\n"
            .into()
    }
}

/// The outside witness in plain words (`alm audit-settings`, Task-A01-wood982).
pub(crate) fn witness_lines(w: Option<&allomos_core::witness::Witness>) -> String {
    match w {
        None => "Outside witness: off. Checkpoints are signed on this computer only; no fingerprint leaves it.\n".into(),
        Some(w) => format!(
            "Outside witness: on. Each checkpoint's fingerprint (a hash, never the content) is sent to {}; the receipt is kept in the checkpoint.\n{}",
            w.services().join(" and "),
            match w.when() {
                allomos_core::witness::When::Daily => "How often: daily. The app while it is open, and any alm command that changes the taskbag, make a checkpoint when the last one is a day old.\n",
                allomos_core::witness::When::Approvals => "How often: at each approval and human check (alm approve, alm checked and the app's same).\n",
                allomos_core::witness::When::Asked => "How often: only when asked (alm checkpoint, Make a checkpoint now, and upgrading).\n",
            }
        ),
    }
}

/// Retention and the legal hold in plain words (`alm audit-settings`, Task-A01-stem986).
pub(crate) fn retention_lines(
    kept: &allomos_core::retention::Retention,
    past: Option<&allomos_store::redact::PastRetention>,
) -> String {
    let mut text = match kept.years {
        None => "Keep the history: forever.\n".to_string(),
        Some(y) => format!(
            "Keep the history: {y} year{}.\n",
            if y == 1 { "" } else { "s" }
        ),
    };
    if let Some(p) = past {
        if p.events == 0 {
            text.push_str(&format!(
                "No event is older than the retention period (before {}).\n",
                p.cutoff
            ));
        } else {
            text.push_str(&format!(
                "Fact: {} event{} older than the retention period (before {}; the oldest from {}). Allomos removes nothing on its own; what to do with them is the project's decision.\n",
                p.events,
                if p.events == 1 { " is" } else { "s are" },
                p.cutoff,
                p.oldest.as_deref().unwrap_or("?")
            ));
        }
    }
    text.push_str("China's Cybersecurity Law (Art. 23) asks for network logs to be kept at least six months.\n");
    text.push_str(if kept.legal_hold {
        "Legal hold: on. Nothing in the history may be removed or redacted while it is on.\n"
    } else {
        "Legal hold: off. Personal details written by mistake can be removed with alm redact.\n"
    });
    text
}

/// One line saying which language to write tasks in, and why.
pub(crate) fn language_line(l: &allomos_store::TaskLanguage) -> String {
    let why = match l.source {
        "taskbag" => "set in this taskbag".to_string(),
        "app" => "follows the Allomos app's language".to_string(),
        _ => "follows this computer's language; the Allomos app has not saved one yet".to_string(),
    };
    format!(
        "Write new and changed tasks in: {} ({why}). Keep IDs, commands, code and file names as they are.",
        l.language
    )
}

/// `alm file-size-settings` (Task-A03-dapper008): show the project's line limit for source files,
/// or change it. alm keeps and shows the number; it measures no file.
fn file_size_settings(
    cli: &Cli,
    cwd: &Path,
    limit: Option<&str>,
    clear: bool,
) -> Result<Done, Problem> {
    use allomos_core::file_size;
    let root = locate(cli, cwd)?;
    let changing = limit.is_some() || clear;
    let (project, issues, changed) = if changing {
        let lock = Lock::acquire(&root, lock_wait())?;
        let (bag, issues) = taskbag_of(&root, lock.load())?;
        let project = file_size::set_limit(&bag.project, limit, clear)?;
        let changed = project != bag.project;
        if changed {
            save_project(
                &lock,
                cli.holder.as_deref(),
                "file-size-settings",
                &bag.project,
                &project,
            )?;
        }
        (project, issues, changed)
    } else {
        let (bag, issues) = taskbag_of(&root, load(&root))?;
        (bag.project, issues, false)
    };
    let applied = file_size::limit(&project);
    let set = project
        .file_size
        .as_ref()
        .and_then(|s| s.limit_lines)
        .is_some();
    let mut text = String::new();
    if changing && !changed {
        text.push_str("Nothing changed: the line limit already says that.\n");
    }
    text.push_str(&format!(
        "Line limit for source files: {applied}{}: tests do not count. A source file over it may only get shorter, never longer, task by task; the project's own tests check that (`alm guide file-size`). alm keeps and shows this number; it measures no file.\n",
        if set { "" } else { " (default)" },
    ));
    let mut done = Done::new(
        json!({
            "taskbag": root,
            "file_size": project.file_size,
            "limit_lines": applied,
            "default_limit_lines": file_size::DEFAULT_LIMIT,
        }),
        text,
    );
    done.issues = issues;
    Ok(done)
}

/// `alm new-task-settings` (Task-A00-wheat560): show whether new tasks stay in backlog until the
/// approver agrees, or turn it on or off. alm keeps the setting and words `alm add task`'s next
/// steps by it; it stops nothing.
fn new_task_settings(cli: &Cli, cwd: &Path, on: Option<bool>) -> Result<Done, Problem> {
    use allomos_core::new_tasks;
    let root = locate(cli, cwd)?;
    let (project, issues, changed) = if let Some(on) = on {
        let lock = Lock::acquire(&root, lock_wait())?;
        let (bag, issues) = taskbag_of(&root, lock.load())?;
        let project = new_tasks::set_wait_for_approval(&bag.project, on);
        let changed = project != bag.project;
        if changed {
            save_project(
                &lock,
                cli.holder.as_deref(),
                "new-task-settings",
                &bag.project,
                &project,
            )?;
        }
        (project, issues, changed)
    } else {
        let (bag, issues) = taskbag_of(&root, load(&root))?;
        (bag.project, issues, false)
    };
    let waits = new_tasks::wait_for_approval(&project);
    let mut text = String::new();
    if on.is_some() && !changed {
        text.push_str("Nothing changed: the setting already says that.\n");
    }
    text.push_str(if waits {
        "New tasks wait for the approver: on. An agent that adds a task leaves it in backlog, tells the approver its ID and makes it ready only once they agree (`alm add task` says so). The exception: in a goal run the agent makes its tasks ready itself, since submitting the goal approved the plan in advance.\n"
    } else {
        "New tasks wait for the approver: off (default). An agent that adds a task makes it ready itself when nothing in it is the person's to decide; otherwise it leaves it in backlog with the open questions in its Notes. Turn it on: `alm new-task-settings --wait-for-approval on`.\n"
    });
    let mut done = Done::new(
        json!({
            "taskbag": root,
            "new_tasks": project.new_tasks,
            "wait_for_approval": waits,
        }),
        text,
    );
    done.issues = issues;
    Ok(done)
}
