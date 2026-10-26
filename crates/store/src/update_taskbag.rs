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
use crate::translate::{target_text, translation_steps_in};
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
    agent_update_taskbag_request_with(language, project, taskbag, None)
}

/// The language the person chose to translate the taskbag into: its code (`vi`, `pt-BR`) and, when
/// the app has it, the name its menu showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target<'a> {
    pub code: &'a str,
    pub name: Option<&'a str>,
}

/// [`agent_update_taskbag_request_in`], with the person's choice of a language to translate the
/// taskbag's contents into (Task-A05-cape996, approver 2026-10-26; it replaces A03-totem655's
/// checkbox, which offered only the interface's language): every task's title, description and
/// handoff note, the card files and the project notes, the translation replacing the old text;
/// commands, task IDs, file paths and code stay as they are; and the task language becomes that
/// language (`alm task-language <code>`). `None` (Don't translate): the request says not to.
pub fn agent_update_taskbag_request_with(
    language: &str,
    project: &Path,
    taskbag: &Path,
    translate: Option<Target>,
) -> String {
    let request = update_request(language, project, taskbag);
    format!(
        "{}\n\n---\n\n{}",
        request.trim_end(),
        translation_part_in(language, taskbag, translate)
    )
}

/// What the request says about translating, in its own language: the same steps as Translate the
/// taskbag… ([`translation_steps_in`]), done after the work above, in the same plan.
fn translation_part_in(language: &str, taskbag: &Path, translate: Option<Target>) -> String {
    let Some(target) = translate else {
        return match language {
            "zh-CN" => "翻译：不要翻译。任务包的内容保持原来的语言。\n",
            "ja" => "翻訳：翻訳しないでください。タスクバッグの内容は書かれている言語のままにします。\n",
            _ => "Translation: do not translate. The taskbag's contents stay in the language they are written in.\n",
        }
        .to_string();
    };
    let name = target_text(target.code, target.name);
    let steps = translation_steps_in(language, taskbag, target.code, target.name);
    match language {
        "zh-CN" => format!(
            "翻译：把任务包的内容翻译成{name}，在上面的工作完成之后做，同样先在计划里写明，用户选择“继续”后才动手。\n{steps}"
        ),
        "ja" => format!(
            "翻訳：タスクバッグの内容を{name}に翻訳してください。上の作業が終わってから行い、同じく先に計画に書き、ユーザーが「進める」を選んでから手を付けます。\n{steps}"
        ),
        _ => format!(
            "Translation: translate the taskbag's contents into {name}, after the work above, as part of the same plan, and only after the person chooses Go ahead.\n{steps}"
        ),
    }
}

fn update_request(language: &str, project: &Path, taskbag: &Path) -> String {
    if let Some(m) = versions::move_in(taskbag) {
        return agent_move_in_request_in(language, project, taskbag, &m);
    }
    let section = versions::check_connected(taskbag).filter(|c| !c.outdated_files().is_empty());
    let cards = !cards::outside(taskbag).is_empty();
    let handoffs = handoff::legacy(taskbag);
    let no_tasks = crate::load(taskbag)
        .taskbag
        .is_some_and(|bag| bag.tasks.is_empty());
    // The upgrade first (Task-A05-fish286): the other jobs then run against the newer format.
    let upgrade = versions::upgradable(taskbag);
    let parts: Vec<String> = [
        upgrade.map(|found| versions::agent_upgrade_request_in(language, project, taskbag, &found)),
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
    fn an_allomos_5_taskbag_is_upgraded_first() {
        let p = project("upgrade");
        let tb = p.join(DEFAULT_DIR);
        init(&tb, "p", "P").unwrap();
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(!r.contains("upgrade --confirm"), "{r}");
        let file = tb.join(crate::PROJECT_FILE);
        let text = fs::read_to_string(&file).unwrap();
        fs::write(&file, text.replace(allomos_core::FORMAT, "allomos/5")).unwrap();
        // The upgrade comes first, then the other jobs (here: looking for the project's tasks).
        let r = agent_update_taskbag_request_in("en", &p, &tb);
        assert!(r.contains("upgrade --confirm"), "{r}");
        assert!(
            r.find("upgrade --confirm").unwrap() < r.find("conversion request").unwrap(),
            "{r}"
        );
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
        // Translating is the person's choice (Task-A03-totem655): unchosen, the request says not to.
        assert!(r.contains("Translation: do not translate."), "{r}");
        // Any language, and the task language becomes it (Task-A05-cape996).
        let vi = Some(Target {
            code: "vi",
            name: Some("Tiếng Việt"),
        });
        for language in ["en", "zh-CN", "ja"] {
            let yes = agent_update_taskbag_request_with(language, &p, &tb, vi);
            assert!(yes.contains("Tiếng Việt (`vi`)"), "{language}: {yes}");
            for kept in ["task-language vi", "tasks/<id>/card.md", "project-notes.md"] {
                assert!(yes.contains(kept), "{language}: {kept}");
            }
            let no = agent_update_taskbag_request_with(language, &p, &tb, None);
            assert!(
                !no.contains("tasks/<id>/card.md") || no.contains("move-cards"),
                "{language}"
            );
            assert!(!no.contains("task-language vi"), "{language}: {no}");
        }
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
