//! App changes awaiting an agent sharing them (decision 0017).

use crate::events::list_with_files;
use allomos_core::event::Kind;
use serde::Serialize;
use std::path::Path;

/// Changes made in the app that no agent has been told to share yet (decision 0017, row 9): the
/// app's events after the newest change made through `alm`, and the files they touch that still
/// exist, relative to the taskbag. The next `alm` change lists them with the Git commands to
/// commit them as the person's; after that change they are no longer "after the newest `alm`
/// change". Allomos never reads Git, so a change someone already committed by hand may be listed
/// once more; committing it again changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AppChanges {
    /// How many changes (events) the app made.
    pub count: usize,
    /// Who made them, as the app recorded it (the person's name), without repeats.
    pub by: Vec<String>,
    /// Their files, relative to the taskbag, with `/`.
    pub files: Vec<String>,
}

pub fn unshared_app_changes(root: &Path) -> AppChanges {
    let (all, _) = list_with_files(root);
    let after = all
        .iter()
        .rposition(|(_, e)| e.via == "alm")
        .map_or(0, |i| i + 1);
    let mut out = AppChanges::default();
    let mut files = std::collections::BTreeSet::new();
    for (file, e) in &all[after..] {
        if e.via != "app" {
            continue;
        }
        out.count += 1;
        if let Some(by) = &e.by {
            if !out.by.contains(by) {
                out.by.push(by.clone());
            }
        }
        files.insert(file.clone());
        let id = e.id.as_deref().unwrap_or_default();
        let item: Vec<String> = match e.kind {
            Kind::Task => vec![
                format!("tasks/{id}/task.json"),
                format!("tasks/{id}/description.md"),
            ],
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
