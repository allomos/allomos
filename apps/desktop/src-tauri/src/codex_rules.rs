//! Monitor ready tasks… with Codex (Task-A03-pier032, approver 2026-11-09): the option that adds
//! Allomos's rules for Codex (`~/.codex/rules/allomos.rules`) so an unattended Codex session can
//! pull, commit and push. The dialog reads the file's state while it is open and writes nothing;
//! only Start adds, updates or removes the file, and each of these is recorded in the project's
//! history (its audit record). The person's own `default.rules` is never touched.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_store::codex_rules::{self, Rule, State};
use serde::Serialize;

use crate::LOCK_WAIT;

/// What the dialog shows: where the file goes, the rules this Allomos ships, and how the file on
/// this computer stands against them.
#[derive(Debug, Serialize)]
pub(crate) struct CodexRules {
    path: String,
    /// The version this Allomos ships (the state's own `version` is the file's).
    shipped: u32,
    rules: Vec<Rule>,
    #[serde(flatten)]
    state: State,
}

fn path() -> Result<PathBuf, Problem> {
    codex_rules::codex_home()
        .map(|h| codex_rules::path_in(&h))
        .ok_or_else(|| Problem::WriteFailed {
            path: "~/.codex/rules".into(),
            detail: "no home folder".into(),
        })
}

fn of(path: &Path) -> CodexRules {
    CodexRules {
        path: path.display().to_string(),
        shipped: codex_rules::VERSION,
        rules: codex_rules::rules(codex_rules::text()),
        state: codex_rules::state(path),
    }
}

#[tauri::command]
pub(crate) fn codex_rules_status() -> Result<CodexRules, Problem> {
    Ok(of(&path()?))
}

/// Start in Monitor ready tasks…: `install` writes this version's rules (adding or replacing the
/// file), `remove` takes Allomos's file away. A newer version's file is left alone. The change is
/// recorded in the taskbag at `root` (command `codex-rules`, field `codex_rules`: the version
/// before and after; `changed` for a file somebody edited), made as the person's saved name.
#[tauri::command]
pub(crate) fn codex_rules_apply(root: PathBuf, action: String) -> Result<CodexRules, Problem> {
    apply(&root, &path()?, &action)
}

fn apply(root: &Path, path: &Path, action: &str) -> Result<CodexRules, Problem> {
    let before = codex_rules::state(path);
    let from = match &before {
        State::Missing => None,
        State::Current => Some(codex_rules::VERSION.into()),
        State::Older { version, .. } => Some((*version).into()),
        State::Newer { .. } => return Ok(of(path)),
        State::Changed { .. } => Some("changed".into()),
    };
    let to = match action {
        "install" if before == State::Current => return Ok(of(path)),
        "install" => Some(codex_rules::VERSION.into()),
        "remove" if before == State::Missing => return Ok(of(path)),
        "remove" => None,
        other => {
            return Err(Problem::WriteFailed {
                path: path.display().to_string(),
                detail: format!("unknown action {other}"),
            });
        }
    };
    let name = allomos_store::saved_owner_name();
    let actor = crate::app_actor(name.as_deref()).unwrap_or(allomos_store::Actor::APP);
    let lock = allomos_store::Lock::acquire(root, LOCK_WAIT)?;
    let loaded = lock.load();
    let bag = loaded
        .taskbag
        .ok_or_else(|| loaded.issues[0].problem.clone())?;
    let event = actor.event(
        "codex-rules",
        allomos_core::event::Kind::Project,
        Some(&bag.project.id),
        vec![allomos_core::event::Change {
            field: "codex_rules".into(),
            from,
            to,
            extra: Default::default(),
        }],
    );
    let failed = |e: std::io::Error| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    };
    lock.record(&event, |_| {
        if action == "install" {
            codex_rules::install(path).map_err(failed)
        } else {
            codex_rules::remove(path).map_err(failed)
        }
    })?;
    Ok(of(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_adds_updates_and_removes_only_allomos_file_and_records_each() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-codex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&root, "p", "P").unwrap();
        let file = codex_rules::path_in(&dir.join("codex"));
        let own = file.with_file_name("default.rules");
        std::fs::create_dir_all(own.parent().unwrap()).unwrap();
        std::fs::write(&own, "mine").unwrap();
        let events = || {
            allomos_store::events::list(&root)
                .0
                .into_iter()
                .filter(|e| e.command == "codex-rules")
                .map(|e| (e.changes[0].from.clone(), e.changes[0].to.clone()))
                .collect::<Vec<_>>()
        };

        // Added, then nothing more on a second Start.
        assert_eq!(
            apply(&root, &file, "install").unwrap().state,
            State::Current
        );
        apply(&root, &file, "install").unwrap();
        assert_eq!(events(), [(None, Some(1.into()))]);

        // A file somebody changed is replaced only on install, and recorded as changed.
        std::fs::write(&file, "edited").unwrap();
        apply(&root, &file, "install").unwrap();
        assert_eq!(events()[1], (Some("changed".into()), Some(1.into())));

        // Removed; the person's own file stays.
        assert_eq!(apply(&root, &file, "remove").unwrap().state, State::Missing);
        assert_eq!(events()[2], (Some(1.into()), None));
        assert_eq!(std::fs::read_to_string(&own).unwrap(), "mine");

        // A newer version is left alone.
        let newer = codex_rules::text().replace("version 1.", "version 9.");
        std::fs::write(&file, &newer).unwrap();
        apply(&root, &file, "install").unwrap();
        apply(&root, &file, "remove").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), newer);
        assert_eq!(events().len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
