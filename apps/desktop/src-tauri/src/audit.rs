//! Project settings → Audit and Settings → This computer (decision 0021 § 6): the commands
//! behind them. The identity level (Task-A01-frolic587), separation of duties
//! (Task-A01-grotto036), retention and legal hold (Task-A01-stem986); the rest of the section
//! and This computer: Task-A01-tail719; Replace key…: Task-A01-ring220.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use serde::Serialize;

use crate::{LOCK_WAIT, load, opener};

/// The project's audit settings for Project settings → Audit (decision 0021 § 6): so far the
/// identity level (Task-A01-frolic587), with where this computer keeps who is behind its
/// pseudonyms.
#[derive(Serialize)]
pub(crate) struct AuditSettings {
    identity: allomos_core::audit::Identity,
    /// Separation of duties (Task-A01-grotto036): off by default.
    separation_of_duties: bool,
    /// Retention (Task-A01-stem986): years to keep the history; null: forever.
    retention_years: Option<u32>,
    /// Legal hold (Task-A01-stem986): nothing in the history may be removed or redacted.
    legal_hold: bool,
    /// `identities.json` in the settings folder; null without one.
    identities_file: Option<String>,
    /// Checkpoints (Task-A01-tail719, shown under Outside witness): how many, and the newest.
    checkpoints: Checkpoints,
    /// The outside witness (Task-A01-wood982): its services; null while it is off.
    witness: Option<allomos_core::witness::Witness>,
    /// How often checkpoints are made by themselves (Task-A01-bake791): `daily`, `approvals` or
    /// `asked`, as this Allomos reads the setting; null while the witness is off.
    witness_when: Option<&'static str>,
    /// The services offered by name, the default first.
    witness_presets: &'static [allomos_core::witness::Preset],
    /// What the last **Make a checkpoint now** sent: receipts added and fingerprints still
    /// waiting; null otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    witnessed: Option<allomos_store::checkpoint::Witnessed>,
    /// Where this computer keeps its signing key: `keychain`, `folder` or `none` (then no
    /// checkpoint can be made here).
    signing: &'static str,
}

/// The taskbag's checkpoints (decision 0021 § 3.2), as Project settings → Audit shows them.
#[derive(Serialize)]
pub(crate) struct Checkpoints {
    count: usize,
    /// The newest: its file and time; null before the first.
    last: Option<LastCheckpoint>,
    /// Time-stamp receipts across all checkpoints (the outside witness, Task-A01-wood982).
    receipts: usize,
    /// Fingerprints waiting to be sent (offline, or the service refused): checkpoint and service.
    waiting: usize,
}

#[derive(Serialize)]
pub(crate) struct LastCheckpoint {
    file: String,
    at: String,
}

fn checkpoints_of(root: &Path) -> Checkpoints {
    let (all, _) = allomos_store::checkpoint::list(root);
    let receipts = all
        .iter()
        .filter_map(|(_, v)| v.get("receipts").and_then(|r| r.as_array()).map(Vec::len))
        .sum();
    // Their names start with the time they were made: the last listed is the newest.
    let last = all.last().map(|(file, v)| LastCheckpoint {
        file: file.clone(),
        at: v
            .get("at")
            .and_then(|a| a.as_str())
            .unwrap_or_default()
            .to_string(),
    });
    Checkpoints {
        count: all.len(),
        last,
        receipts,
        waiting: allomos_store::checkpoint::waiting(root).len(),
    }
}

pub(crate) fn audit_settings_of(root: &Path) -> Result<AuditSettings, Problem> {
    let loaded = load(root);
    let Some(bag) = loaded.taskbag else {
        return Err(loaded.issues.first().map(|i| i.problem.clone()).unwrap_or(
            Problem::NotATaskbag {
                path: root.display().to_string(),
            },
        ));
    };
    Ok(AuditSettings {
        identity: allomos_core::audit::identity(&bag.project),
        separation_of_duties: allomos_core::duties::on(&bag.project),
        retention_years: allomos_core::retention::of(&bag.project).years,
        legal_hold: allomos_core::retention::of(&bag.project).legal_hold,
        identities_file: allomos_store::identity::identities_file()
            .map(|p| p.display().to_string()),
        checkpoints: checkpoints_of(root),
        signing: allomos_store::this_computer().store,
        witness: allomos_core::witness::of(&bag.project),
        witness_when: allomos_core::witness::of(&bag.project).map(|w| w.when().id()),
        witness_presets: &allomos_core::witness::PRESETS,
        witnessed: None,
    })
}

/// **Make a checkpoint now** (Task-A01-tail719), as `alm checkpoint` does: a signed fingerprint
/// of the whole taskbag; with the outside witness on (Task-A01-wood982), every waiting
/// fingerprint is then sent, off the window's thread. Refused without a signing key on this
/// computer. Returns the settings, with what was sent.
#[tauri::command]
pub(crate) async fn make_checkpoint(root: PathBuf) -> Result<AuditSettings, Problem> {
    tauri::async_runtime::spawn_blocking(move || make_checkpoint_at(&root))
        .await
        .map_err(|e| Problem::WriteFailed {
            path: String::new(),
            detail: e.to_string(),
        })?
}

pub(crate) fn make_checkpoint_at(root: &Path) -> Result<AuditSettings, Problem> {
    let (_, witnessed) = allomos_store::checkpoint::checkpoint(root, LOCK_WAIT, None)?;
    Ok(AuditSettings {
        witnessed: Some(witnessed),
        ..audit_settings_of(root)?
    })
}

/// The window's clock (Task-A01-bake791): while a project is open, the window asks every few
/// minutes; with the outside witness on and set to daily, a checkpoint is made when the last one
/// is a day old (decided again under the lock, so the app and `alm` make one between them), and
/// waiting fingerprints are tried again. Off the window's thread. Null: nothing was due.
#[tauri::command]
pub(crate) async fn witness_tick(
    root: PathBuf,
) -> Result<Option<allomos_store::cadence::Auto>, Problem> {
    tauri::async_runtime::spawn_blocking(move || {
        allomos_store::cadence::auto(&root, LOCK_WAIT, allomos_store::cadence::Trigger::Change)
    })
    .await
    .map_err(|e| Problem::WriteFailed {
        path: String::new(),
        detail: e.to_string(),
    })?
}

/// After the app's Approve and its human check (Task-A01-bake791): with the outside witness set
/// to approvals (or daily and due), a checkpoint, made and sent on a thread of its own so the
/// window does not wait for the network. A refusal (no signing key) changes nothing.
pub(crate) fn after_approval(root: &Path) {
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let _ = allomos_store::cadence::auto(
            &root,
            LOCK_WAIT,
            allomos_store::cadence::Trigger::Approval,
        );
    });
}

/// Turn the outside witness on or off, or change its services (Task-A01-wood982), as `alm
/// audit-settings --witness` does: recorded as an `audit-settings` event. Turning it on needs
/// `confirmed` (the window's confirmation: only the fingerprint leaves the computer); off needs a
/// reason. `on` null: keep it on and change only the services. Returns the settings as saved.
#[tauri::command]
pub(crate) fn set_witness(
    root: PathBuf,
    on: Option<bool>,
    service: Option<String>,
    second: Option<String>,
    when: Option<String>,
    confirmed: bool,
    reason: Option<String>,
) -> Result<AuditSettings, Problem> {
    let change = allomos_core::witness::WitnessChange {
        on,
        service: service.as_deref(),
        second: second.as_deref(),
        when: when.as_deref(),
        confirmed,
        reason: reason.as_deref(),
    };
    set_witness_at(&root, &change)
}

pub(crate) fn set_witness_at(
    root: &Path,
    change: &allomos_core::witness::WitnessChange,
) -> Result<AuditSettings, Problem> {
    let lock = allomos_store::Lock::acquire(root, LOCK_WAIT)?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let (project, changes) = allomos_core::witness::set(&bag.project, change)?;
    if !changes.is_empty() {
        let event = allomos_store::Actor::APP.event(
            "audit-settings",
            allomos_core::event::Kind::Project,
            Some(&project.id),
            changes,
        );
        lock.record(&event, |l| l.save_project(&project))?;
    }
    drop(lock);
    audit_settings_of(root)
}

/// What **Remove personal details…** did: the redaction, or null when nothing was to remove.
#[derive(Serialize)]
pub(crate) struct RedactDone {
    redaction: Option<allomos_core::retention::Redaction>,
}

/// **Remove personal details…** (Task-A01-tail719), as `alm redact` does (Task-A01-stem986):
/// every past value of `fields` in task `id`'s history that differs from the current one is
/// removed, and one `redact` event says which fields and files, who approved it and why, never
/// the old values. Refused while a legal hold is on, without a field or a reason.
#[tauri::command]
pub(crate) fn redact_task(
    root: PathBuf,
    id: String,
    fields: Vec<String>,
    reason: Option<String>,
    approved_by: Option<String>,
) -> Result<RedactDone, Problem> {
    let approved_by = approved_by.filter(|a| !a.trim().is_empty());
    let r = allomos_store::redact::redact(
        &root,
        allomos_store::Actor::APP,
        LOCK_WAIT,
        id.trim(),
        &allomos_store::redact::Request {
            fields: &fields,
            events: &[],
            reason: reason.as_deref(),
            approved_by: approved_by.as_deref(),
        },
    )?;
    Ok(RedactDone {
        redaction: r.redaction,
    })
}

/// Settings → This computer (Task-A01-tail719, decision 0021 § 6).
#[derive(Serialize)]
pub(crate) struct ThisComputerView {
    #[serde(flatten)]
    signing: allomos_store::ThisComputer,
    /// Last seen history: one line per project this computer remembers a checkpoint of.
    seen: Vec<SeenLine>,
}

#[derive(Serialize)]
pub(crate) struct SeenLine {
    project: String,
    checkpoint: String,
    seen_at: String,
    /// The recent folder whose taskbag is this project, when one is.
    folder: Option<String>,
    /// Whether that taskbag still has the checkpoint; null when no recent folder is this project.
    still_there: Option<bool>,
}

/// The key and the checkpoints seen, matched to the recent folders the window passes.
#[tauri::command]
pub(crate) fn this_computer(folders: Vec<String>) -> ThisComputerView {
    this_computer_with(&folders)
}

fn this_computer_with(folders: &[String]) -> ThisComputerView {
    let roots: Vec<(String, PathBuf, Option<String>)> = folders
        .iter()
        .filter_map(|f| match allomos_store::inspect(Path::new(f)) {
            allomos_store::Inspection::Found { root } => {
                let id = load(&root).taskbag.map(|b| b.project.id);
                Some((f.clone(), root, id))
            }
            _ => None,
        })
        .collect();
    let seen = allomos_store::checkpoint::seen()
        .into_iter()
        .map(|s| {
            let here = roots
                .iter()
                .find(|(_, _, id)| id.as_deref() == Some(s.project.as_str()));
            SeenLine {
                folder: here.map(|(f, _, _)| f.clone()),
                still_there: here.map(|(_, root, _)| root.join(&s.checkpoint).is_file()),
                project: s.project,
                checkpoint: s.checkpoint,
                seen_at: s.seen_at,
            }
        })
        .collect();
    ThisComputerView {
        signing: allomos_store::this_computer(),
        seen,
    }
}

/// **Replace key…** (Task-A01-ring220, decision 0021 § 6): a new signing key for this computer;
/// in each taskbag of `folders` (the recent folders the window passes) that has the old key, the
/// old one is retired and the new one added, each with its signed event. Returns what was done
/// per taskbag; folders that are not a taskbag are left out.
#[tauri::command]
pub(crate) fn replace_key(folders: Vec<String>) -> Result<allomos_store::Replaced, Problem> {
    let roots = taskbag_roots(&folders);
    allomos_store::replace_key(&roots, allomos_store::Actor::APP, LOCK_WAIT)
}

/// The taskbag of each folder that has one, each once.
fn taskbag_roots(folders: &[String]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for f in folders {
        if let allomos_store::Inspection::Found { root } = allomos_store::inspect(Path::new(f)) {
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
    roots
}

/// **Export public key…**: write this computer's public key file where the person chose.
#[tauri::command]
pub(crate) fn export_public_key(path: PathBuf) -> Result<(), Problem> {
    let key = allomos_store::this_computer()
        .key
        .ok_or(Problem::NoSigningKey)?;
    std::fs::write(&path, allomos_store::public_key_export(&key)).map_err(|e| {
        Problem::WriteFailed {
            path: path.display().to_string(),
            detail: e.to_string(),
        }
    })
}

#[tauri::command]
pub(crate) fn audit_settings(root: PathBuf) -> Result<AuditSettings, Problem> {
    audit_settings_of(&root)
}

/// Turn separation of duties on or off (Task-A01-grotto036), as `alm audit-settings
/// --separation-of-duties` does: recorded as an `audit-settings` event; turning it off needs a
/// reason. Returns the settings as saved.
#[tauri::command]
pub(crate) fn set_separation(
    root: PathBuf,
    on: bool,
    reason: Option<String>,
) -> Result<AuditSettings, Problem> {
    set_separation_at(&root, on, reason.as_deref())
}

pub(crate) fn set_separation_at(
    root: &Path,
    on: bool,
    reason: Option<&str>,
) -> Result<AuditSettings, Problem> {
    let lock = allomos_store::Lock::acquire(root, LOCK_WAIT)?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let (project, changes) = allomos_core::duties::set(&bag.project, on, reason)?;
    let event = allomos_store::Actor::APP.event(
        "audit-settings",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        changes,
    );
    lock.record(&event, |l| l.save_project(&project))?;
    drop(lock);
    audit_settings_of(root)
}

/// Change the retention period and the legal hold (Task-A01-stem986), as `alm audit-settings
/// --retention --legal-hold` does: recorded as an `audit-settings` event; a shorter period and
/// turning the hold on or off need a reason. `years` null: forever. Returns the settings as saved.
#[tauri::command]
pub(crate) fn set_retention(
    root: PathBuf,
    years: Option<u32>,
    legal_hold: bool,
    reason: Option<String>,
) -> Result<AuditSettings, Problem> {
    set_retention_at(&root, years, legal_hold, reason.as_deref())
}

pub(crate) fn set_retention_at(
    root: &Path,
    years: Option<u32>,
    legal_hold: bool,
    reason: Option<&str>,
) -> Result<AuditSettings, Problem> {
    if let Some(y) = years {
        allomos_core::retention::parse(&y.to_string())?;
    }
    let lock = allomos_store::Lock::acquire(root, LOCK_WAIT)?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let change = allomos_core::retention::RetentionChange {
        years: Some(years),
        legal_hold: Some(legal_hold),
        reason,
    };
    let (project, changes) = allomos_core::retention::set(&bag.project, &change)?;
    let event = allomos_store::Actor::APP.event(
        "audit-settings",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        changes,
    );
    lock.record(&event, |l| l.save_project(&project))?;
    drop(lock);
    audit_settings_of(root)
}

/// Change the identity level, as `alm audit-settings` does, recorded as an `audit-settings`
/// event with the reason. Returns the settings as saved.
#[tauri::command]
pub(crate) fn set_identity(
    root: PathBuf,
    level: Option<String>,
    reauthenticate: Option<bool>,
    reason: Option<String>,
) -> Result<AuditSettings, Problem> {
    set_identity_at(&root, level.as_deref(), reauthenticate, reason.as_deref())
}

pub(crate) fn set_identity_at(
    root: &Path,
    level: Option<&str>,
    reauthenticate: Option<bool>,
    reason: Option<&str>,
) -> Result<AuditSettings, Problem> {
    let lock = allomos_store::Lock::acquire(root, LOCK_WAIT)?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let change = allomos_core::audit::IdentityChange {
        level,
        reauthenticate,
        reason,
    };
    let (project, changes) = allomos_core::audit::set_identity(&bag, &change)?;
    let event = allomos_store::Actor::APP.event(
        "audit-settings",
        allomos_core::event::Kind::Project,
        Some(&project.id),
        changes,
    );
    lock.record(&event, |l| l.save_project(&project))?;
    drop(lock);
    audit_settings_of(root)
}

/// The user guide's standards map (Task-A01-gnome953), in the window's language: which settings
/// each standard needs. Fixed addresses: the window cannot ask to open any other.
pub(crate) fn standards_map_url(language: &str) -> &'static str {
    match language {
        "zh-CN" => "https://allomos.github.io/allomos/zh-cn/audit/standards-map/",
        "ja" => "https://allomos.github.io/allomos/ja/audit/standards-map/",
        _ => "https://allomos.github.io/allomos/audit/standards-map/",
    }
}

/// Project settings → Audit → See which settings each standard needs: the standards map in the
/// default browser.
#[tauri::command]
pub(crate) fn open_standards_map(language: String) -> Result<(), Problem> {
    let url = standards_map_url(&language);
    opener()
        .arg(url)
        .spawn()
        .map(drop)
        .map_err(|e| Problem::OpenFailed {
            path: url.to_string(),
            detail: e.to_string(),
        })
}

/// Show `identities.json` (who is behind this computer's pseudonyms) in Finder or Explorer:
/// its folder, which is made if missing.
#[tauri::command]
pub(crate) fn show_identities_file() -> Result<(), Problem> {
    let dir = allomos_store::settings_dir().ok_or_else(|| Problem::OpenFailed {
        path: String::new(),
        detail: "This computer has no settings folder for Allomos.".into(),
    })?;
    let _ = std::fs::create_dir_all(&dir);
    let mut command = opener();
    command.arg(allomos_store::plain_path(&dir));
    command.spawn().map(drop).map_err(|e| Problem::OpenFailed {
        path: dir.display().to_string(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_store::init;

    /// The standards map's address follows the window's language; any other value gets English.
    #[test]
    fn standards_map_in_each_language() {
        assert!(standards_map_url("en").ends_with("/allomos/audit/standards-map/"));
        assert!(standards_map_url("zh-CN").ends_with("/allomos/zh-cn/audit/standards-map/"));
        assert!(standards_map_url("ja").ends_with("/allomos/ja/audit/standards-map/"));
        assert_eq!(standards_map_url("fr"), standards_map_url("en"));
    }

    /// Project settings → Audit → Separation of duties (Task-A01-grotto036): off by default; on
    /// and off are recorded, and off needs a reason.
    #[test]
    fn separation_of_duties_is_off_until_turned_on() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-duties-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        assert!(!audit_settings_of(&root).unwrap().separation_of_duties);
        let count = || std::fs::read_dir(root.join("events")).unwrap().count();
        let before = count();
        assert!(
            set_separation_at(&root, true, None)
                .unwrap()
                .separation_of_duties
        );
        assert_eq!(count(), before + 1);
        assert_eq!(
            set_separation_at(&root, false, None).err(),
            Some(Problem::AuditReasonMissing {
                setting: "separation_of_duties".into()
            })
        );
        assert!(
            !set_separation_at(&root, false, Some("one person"))
                .unwrap()
                .separation_of_duties
        );
        assert_eq!(count(), before + 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Project settings → Audit → Retention (Task-A01-stem986): forever and no hold by default; a
    /// shorter period and the hold need a reason.
    #[test]
    fn retention_and_legal_hold_are_set_with_a_reason() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-retention-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let s = audit_settings_of(&root).unwrap();
        assert_eq!((s.retention_years, s.legal_hold), (None, false));
        assert_eq!(
            set_retention_at(&root, Some(7), false, None).err(),
            Some(Problem::AuditReasonMissing {
                setting: "retention".into()
            })
        );
        let s = set_retention_at(&root, Some(7), true, Some("policy and a case")).unwrap();
        assert_eq!((s.retention_years, s.legal_hold), (Some(7), true));
        assert!(set_retention_at(&root, Some(0), true, Some("x")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Project settings → Audit → Identity (Task-A01-frolic587): a local-only project shows the
    /// pseudonym, refuses the sign-in levels, and records nothing when nothing changes.
    #[test]
    fn a_local_project_keeps_the_pseudonym_identity() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let s = audit_settings_of(&root).unwrap();
        assert_eq!(s.identity.level, "pseudonym");
        assert!(!s.identity.sign_in_available && !s.identity.reauthenticate);
        assert_eq!(
            set_identity_at(&root, Some("sso"), None, None).err(),
            Some(Problem::IdentityNeedsCloud {
                level: "sso".into()
            })
        );
        assert_eq!(
            set_identity_at(&root, None, Some(true), None).err(),
            Some(Problem::ReauthenticateNeedsSignIn)
        );
        let count = || std::fs::read_dir(root.join("events")).unwrap().count();
        let before = count();
        set_identity_at(&root, Some("pseudonym"), Some(false), None).unwrap();
        assert_eq!(count(), before);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The rest of Project settings → Audit (Task-A01-tail719): the checkpoints shown, Make a
    /// checkpoint now refused without a key store (tests never use the Keychain), and Remove
    /// personal details recorded like `alm redact`, refused under a legal hold.
    #[test]
    fn checkpoints_and_remove_personal_details() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-audit-rest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let s = audit_settings_of(&root).unwrap();
        assert_eq!((s.checkpoints.count, s.signing), (0, "none"));
        assert!(s.checkpoints.last.is_none());
        assert_eq!(make_checkpoint_at(&root).err(), Some(Problem::NoSigningKey));
        assert_eq!(s.witness, None);
        assert_eq!(s.checkpoints.waiting, 0);
        // The outside witness (Task-A01-wood982): on only with the confirmation, off with a reason.
        let on = allomos_core::witness::WitnessChange {
            on: Some(true),
            ..Default::default()
        };
        assert_eq!(
            set_witness_at(&root, &on).err(),
            Some(Problem::WitnessNotConfirmed)
        );
        let s = set_witness_at(
            &root,
            &allomos_core::witness::WitnessChange {
                confirmed: true,
                ..on.clone()
            },
        )
        .unwrap();
        assert_eq!(s.witness.unwrap().service, "https://freetsa.org/tsr");
        let off = allomos_core::witness::WitnessChange {
            on: Some(false),
            ..Default::default()
        };
        assert!(set_witness_at(&root, &off).is_err());
        let s = set_witness_at(
            &root,
            &allomos_core::witness::WitnessChange {
                reason: Some("trial over"),
                ..off
            },
        )
        .unwrap();
        assert_eq!(s.witness, None);

        let lock = allomos_store::Lock::acquire(&root, std::time::Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&allomos_core::ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let task = allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap();
        lock.create_task(&task).unwrap();
        drop(lock);
        let note = |text: &str| {
            allomos_store::change_task(&root, allomos_store::Actor::APP, LOCK_WAIT, |bag| {
                allomos_core::ops::edit(
                    bag,
                    "A01-01",
                    Some("Will"),
                    &allomos_core::ops::Edit::HandoffNote {
                        text: Some(text.into()),
                    },
                )
            })
            .unwrap();
        };
        note("call me at <PHONE>");
        note("fixed");
        let fields = vec!["handoff_note".to_string()];
        assert_eq!(
            redact_task(root.clone(), "A01-01".into(), fields.clone(), None, None).err(),
            Some(Problem::RedactReasonMissing)
        );
        set_retention_at(&root, None, true, Some("a case")).unwrap();
        assert!(matches!(
            redact_task(
                root.clone(),
                "A01-01".into(),
                fields.clone(),
                Some("a phone number".into()),
                None
            ),
            Err(Problem::LegalHoldOn { .. })
        ));
        set_retention_at(&root, None, false, Some("the case ended")).unwrap();
        let done = redact_task(
            root.clone(),
            " A01-01 ".into(),
            fields.clone(),
            Some("a phone number".into()),
            Some(" ".into()),
        )
        .unwrap();
        let rd = done.redaction.expect("the old note was removed");
        assert_eq!(rd.fields, fields);
        assert!(rd.approved_by.is_none());
        let text = std::fs::read_dir(root.join("events"))
            .unwrap()
            .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
            .collect::<String>();
        assert!(!text.contains("<PHONE>"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Replace key… (Task-A01-ring220): each taskbag once, folders without one left out; refused
    /// without a key store (tests never use the person's Keychain or Credential Manager).
    #[test]
    fn replace_key_needs_a_key_and_takes_each_taskbag_once() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-replace-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join(allomos_store::DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let folder = dir.display().to_string();
        let nowhere = dir.join("nowhere").display().to_string();
        assert_eq!(
            taskbag_roots(&[folder.clone(), nowhere, folder.clone()]).len(),
            1
        );
        assert_eq!(replace_key(vec![folder]).err(), Some(Problem::NoSigningKey));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
