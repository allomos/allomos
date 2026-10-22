//! What **Update the taskbag…** asks an agent to do, as one request (approver 2026-10-17: one
//! request covers every update due). The taskbag needs updating only in these cases (approver
//! 2026-10-18, Task-A03-27): card files kept outside the taskbag, an older taskbag whose tasks are
//! moved into a new one, an out-of-date Allomos section in the project's instruction files, and
//! old handoff files in `handoffs/` (Task-A03-polar770, approver 2026-10-22).
//! An agent not yet listed under `## Agents set up` is not one of them: each agent registers itself
//! when it starts work (the standing rules in AGENTS.md, `alm guide agent-files`).

use std::path::Path;

use crate::cards;
use crate::convert::{agent_convert_request_in, agent_move_in_request_in};
use crate::handoff;
use crate::versions;

/// The request, in an interface language (`en`, `zh-CN`, `ja`; English otherwise):
/// - an older taskbag's tasks to move in ([`agent_move_in_request_in`], which renews the section
///   too); else
/// - an out-of-date section ([`versions::agent_update_request_in`]), card files kept outside the
///   taskbag ([`cards::agent_move_cards_request_in`]) when some are known, old handoff files
///   ([`handoff::agent_move_handoffs_request_in`]) when there are some, and, for a taskbag with
///   no tasks yet, looking for the tasks the project already keeps ([`agent_convert_request_in`];
///   approver 2026-10-19: the wizard's step 4 asks for it), each that applies; else
/// - with none of them: card files no link names (the move-cards request). File → Update the
///   Taskbag… opens it at any time.
pub fn agent_update_taskbag_request_in(language: &str, project: &Path, taskbag: &Path) -> String {
    if let Some(m) = versions::move_in(taskbag) {
        return agent_move_in_request_in(language, project, taskbag, &m);
    }
    let section = versions::check_connected(taskbag).filter(|c| !c.outdated_files().is_empty());
    let cards = !cards::outside(taskbag).is_empty();
    let handoffs = handoff::legacy(taskbag);
    let no_tasks = crate::load(taskbag)
        .taskbag
        .is_some_and(|bag| bag.tasks.is_empty());
    let parts: Vec<String> = [
        section.map(|c| versions::agent_update_request_in(language, project, taskbag, &c)),
        cards.then(|| cards::agent_move_cards_request_in(language, project, taskbag)),
        (!handoffs.is_empty()).then(|| {
            handoff::agent_move_handoffs_request_in(language, project, taskbag, &handoffs)
        }),
        no_tasks.then(|| agent_convert_request_in(language, project, taskbag)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !parts.is_empty() {
        return parts
            .iter()
            .map(|p| p.trim_end())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
            + "\n";
    }
    cards::agent_move_cards_request_in(language, project, taskbag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, init};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn project(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-update-taskbag-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn the_request_covers_what_is_due() {
        let p = project("update");
        let tb = p.join(DEFAULT_DIR);
        init(&tb, "p", "P").unwrap();
        // No tasks yet, nothing else due: look for the project's own tasks.
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(r.contains("conversion request"), "{r}");
        assert!(!r.contains("agent setup request"), "{r}");
        // No tasks and an out-of-date section: both.
        fs::write(p.join("AGENTS.md"), "## Tasks: Allomos (v1)\n\nalm next\n").unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(
            r.contains("conversion request") && r.contains("AGENTS.md"),
            "{r}"
        );
        fs::remove_file(p.join("AGENTS.md")).unwrap();
        // With a task: the agent looks for card files no link names.
        crate::add_area(
            &tb,
            crate::Actor::APP,
            std::time::Duration::from_secs(1),
            Some("A1"),
            "Area",
        )
        .unwrap();
        crate::add_task(
            &tb,
            crate::Actor::APP,
            std::time::Duration::from_secs(1),
            &serde_json::from_str(r#"{"id":"A1-01","area":"A1","title":"Bake"}"#).unwrap(),
        )
        .unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(
            r.contains("move-cards request") && !r.contains("conversion request"),
            "{r}"
        );
        // An out-of-date section: the update request, and the card files only when some are known.
        fs::write(p.join("AGENTS.md"), "## Tasks: Allomos (v1)\n\nalm next\n").unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(
            r.contains("AGENTS.md") && !r.contains("move-cards request"),
            "{r}"
        );
        fs::create_dir_all(p.join("docs")).unwrap();
        fs::write(p.join("docs/A1-01.md"), "# Card\n").unwrap();
        cards::report(
            &tb,
            crate::Actor::APP,
            std::time::Duration::from_secs(1),
            &[("A1-01".to_string(), "docs/A1-01.md".to_string())],
        )
        .unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(
            r.contains("AGENTS.md") && r.contains("move-cards request"),
            "{r}"
        );
        assert!(r.contains("\n---\n"), "{r}");
        // Old handoff files: their request too (Task-A03-polar770).
        assert!(!r.contains("move-handoffs request"), "{r}");
        fs::create_dir_all(tb.join("handoffs")).unwrap();
        fs::write(tb.join("handoffs/old.md"), "x").unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(
            r.contains("move-handoffs request") && r.contains("`handoffs/old.md`"),
            "{r}"
        );
    }
}
