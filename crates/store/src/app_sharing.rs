//! App changes awaiting an agent sharing them (decision 0017).

use crate::events::list_with_files;
use allomos_core::event::Kind;
use serde::Serialize;
use std::path::Path;

/// Changes made in the app after the furthest explicitly confirmed app event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AppChanges {
    /// Stable event file name, including for an event in a packed month.
    pub through: Option<String>,
    pub entries: Vec<AppChange>,
    /// How many changes (events) the app made.
    pub count: usize,
    /// Who made them, as the app recorded it (the person's name), without repeats.
    pub by: Vec<String>,
    /// Their files, relative to the taskbag, with `/`.
    pub files: Vec<String>,
}

pub fn unshared_app_changes(root: &Path) -> AppChanges {
    let (all, _) = list_with_files(root);
    let after = shared_position(&all).map_or(0, |i| i + 1);
    let mut out = AppChanges::default();
    let mut files = std::collections::BTreeSet::new();
    for (file, e) in &all[after..] {
        if e.via != "app" {
            continue;
        }
        out.count += 1;
        out.through = Some(file.clone());
        out.entries.push(AppChange {
            event: file.clone(),
            at: e.at.clone(),
            id: e.id.clone(),
            command: e.command.clone(),
            fields: e.changes.iter().map(|c| c.field.clone()).collect(),
        });
        if let Some(by) = &e.by {
            if !out.by.contains(by) {
                out.by.push(by.clone());
            }
        }
        // Packed events keep their original name in history but share the containing file.
        files.insert(if root.join(file).is_file() {
            file.clone()
        } else {
            format!("events/{}.{}", &e.at[..7], crate::events::PACK_EXT)
        });
        let id = e.id.as_deref().unwrap_or_default();
        let item: Vec<String> = match e.kind {
            Kind::Task => {
                let mut task_files = vec![format!("tasks/{id}/task.json")];
                if e.changes.iter().any(|c| c.field == "description") {
                    task_files.push(format!("tasks/{id}/description.md"));
                }
                task_files
            }
            Kind::Area => vec![format!("areas/{id}.json")],
            Kind::Iteration => vec![format!("iterations/{id}.json")],
            Kind::Project => vec!["project.json".to_string()],
            Kind::Unknown => Vec::new(),
        };
        for f in item {
            // A file the app removed (a deleted area or iteration, an emptied description) is
            // listed too, so the commit records the removal.
            let removed = match e.kind {
                Kind::Area | Kind::Iteration => e.command.starts_with("delete"),
                Kind::Task => {
                    f.ends_with("description.md")
                        && e.changes
                            .iter()
                            .any(|c| c.field == "description" && c.from.is_some() && c.to.is_none())
                }
                Kind::Project | Kind::Unknown => false,
            };
            if removed || root.join(&f).is_file() {
                files.insert(f);
            }
        }
    }
    out.files = files.into_iter().collect();
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppChange {
    pub event: String,
    pub at: String,
    pub id: Option<String>,
    pub command: String,
    pub fields: Vec<String>,
}

fn shared_position(all: &[(String, allomos_core::event::Event)]) -> Option<usize> {
    all.iter()
        .filter(|(_, e)| e.via == "alm" && e.command == "shared")
        .flat_map(|(_, e)| &e.changes)
        .filter(|c| c.field == "shared_through")
        .filter_map(|c| c.to.as_ref()?.as_str())
        .filter_map(|target| {
            all.iter()
                .position(|(file, e)| file == target && e.via == "app")
        })
        .max()
}

/// Record the agent's assertion, without inspecting Git. Older confirmations never move the
/// boundary backwards. Invalid event references are refused as invalid input.
pub fn confirm(
    root: &Path,
    actor: crate::Actor,
    wait: std::time::Duration,
    through: &str,
) -> Result<bool, allomos_core::Problem> {
    use allomos_core::{Problem, event::Change};
    let (lock, bag, _) = crate::lock_for_change(root, wait)?;
    let (all, _) = list_with_files(root);
    let target = all
        .iter()
        .position(|(file, e)| file == through && e.via == "app")
        .ok_or_else(|| Problem::ClassifyInvalid {
            field: "app event (--through)".into(),
            value: through.into(),
            allowed: "an app event file from app_changes.through".into(),
        })?;
    if shared_position(&all).is_some_and(|i| i >= target) {
        return Ok(false);
    }
    let event = actor.event(
        "shared",
        Kind::Project,
        Some(&bag.project.id),
        vec![Change {
            field: "shared_through".into(),
            from: None,
            to: Some(through.into()),
            extra: Default::default(),
        }],
    );
    lock.record(&event, |_| Ok(()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Actor, Lock};
    use allomos_core::event::Change;
    use std::time::Duration;

    fn fixture() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "allomos-sharing-{}",
            crate::events::random_suffix()
        ));
        crate::init(&root, "test", "Test").unwrap();
        std::fs::remove_dir_all(root.join("events")).unwrap();
        std::fs::create_dir(root.join("events")).unwrap();
        root
    }

    fn event(root: &Path, actor: Actor, n: u8) -> String {
        let before = list_with_files(root).0;
        let mut e = actor.event(
            "title",
            Kind::Project,
            Some("test"),
            vec![Change {
                field: "name".into(),
                from: None,
                to: Some(format!("Title {n}").into()),
                extra: Default::default(),
            }],
        );
        e.at = format!("2026-10-01T00:00:{n:02}Z");
        Lock::acquire(root, Duration::ZERO)
            .unwrap()
            .record(&e, |_| Ok(()))
            .unwrap();
        list_with_files(root)
            .0
            .into_iter()
            .find(|(f, _)| !before.iter().any(|(old, _)| old == f))
            .unwrap()
            .0
    }

    #[test]
    fn confirmation_is_explicit_bounded_and_monotonic() {
        let root = fixture();
        let a = event(&root, Actor::APP, 1);
        event(&root, Actor::alm(Some("agent")), 2);
        let b = event(&root, Actor::APP, 3);
        assert_eq!(unshared_app_changes(&root).count, 2);
        confirm(&root, Actor::alm(Some("agent")), Duration::ZERO, &a).unwrap();
        let left = unshared_app_changes(&root);
        assert_eq!(left.count, 1);
        assert_eq!(left.through.as_deref(), Some(b.as_str()));
        assert!(
            confirm(
                &root,
                Actor::alm(None),
                Duration::ZERO,
                "events/missing.json"
            )
            .is_err()
        );
        confirm(&root, Actor::alm(None), Duration::ZERO, &b).unwrap();
        assert!(!confirm(&root, Actor::alm(None), Duration::ZERO, &a).unwrap());
        assert_eq!(unshared_app_changes(&root).count, 0);
        event(&root, Actor::APP, 4);
        assert_eq!(unshared_app_changes(&root).count, 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn packed_history_keeps_the_boundary_and_names_the_actual_file() {
        let root = fixture();
        let a = event(&root, Actor::APP, 1);
        let b = event(&root, Actor::APP, 2);
        // Build a pack with the same shape as events::pack, without changing the process clock.
        let all = list_with_files(&root).0;
        let lines = all
            .iter()
            .map(|(file, e)| {
                serde_json::json!({"file":file.trim_start_matches("events/"), "event":e})
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(root.join("events/2026-10.events"), lines + "\n").unwrap();
        for (file, _) in &all {
            std::fs::remove_file(root.join(file)).unwrap();
        }
        assert!(
            unshared_app_changes(&root)
                .files
                .contains(&"events/2026-10.events".into())
        );
        confirm(&root, Actor::alm(None), Duration::ZERO, &a).unwrap();
        assert_eq!(unshared_app_changes(&root).through, Some(b.clone()));
        confirm(&root, Actor::alm(None), Duration::ZERO, &b).unwrap();
        assert_eq!(unshared_app_changes(&root).count, 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
