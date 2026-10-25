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
    agent_update_taskbag_request_with(language, project, taskbag, false)
}

/// [`agent_update_taskbag_request_in`], with the person's choice whether the agent also translates
/// the taskbag's contents into the interface's language `language` (Task-A03-totem655, approver
/// 2026-10-25): every task's title, description and handoff note, the card files and the project
/// notes, the translation replacing the old text; commands, task IDs, file paths and code stay as
/// they are, and the task language (`alm task-language`) does not change. Unchosen, the request
/// says not to translate.
pub fn agent_update_taskbag_request_with(
    language: &str,
    project: &Path,
    taskbag: &Path,
    translate: bool,
) -> String {
    let request = update_request(language, project, taskbag);
    format!(
        "{}\n\n---\n\n{}",
        request.trim_end(),
        translation_part_in(language, translate)
    )
}

/// The language as people who speak it name it.
fn language_name(language: &str) -> &'static str {
    match language {
        "zh-CN" => "简体中文",
        "ja" => "日本語",
        _ => "English",
    }
}

/// What the request says about translating, in its own language.
fn translation_part_in(language: &str, translate: bool) -> String {
    let name = language_name(language);
    match (language, translate) {
        ("zh-CN", false) => "翻译：不要翻译。任务包的内容保持原来的语言。\n".to_string(),
        ("zh-CN", true) => format!(
            "翻译：把任务包的内容翻译成{name}，在上面的工作完成之后做，同样先在计划里写明，用户选择“继续”后才动手。\n- 翻译：每个任务的标题、描述和交接备注（`alm title`、`alm describe`、`alm handoff`），卡片文件（`tasks/<id>/card.md`），以及任务包的 project-notes.md。\n- 保持原样：命令、任务 ID、文件路径和代码。\n- 译文取代原文，不保留原文。\n- 任务语言（`alm task-language` 打印的）不变，不要改它。\n"
        ),
        ("ja", false) => "翻訳：翻訳しないでください。タスクバッグの内容は書かれている言語のままにします。\n".to_string(),
        ("ja", true) => format!(
            "翻訳：タスクバッグの内容を{name}に翻訳してください。上の作業が終わってから行い、同じく先に計画に書き、ユーザーが「進める」を選んでから手を付けます。\n- 翻訳するもの：各タスクのタイトル、説明、引き継ぎメモ（`alm title`、`alm describe`、`alm handoff`）、カードのファイル（`tasks/<id>/card.md`）、タスクバッグの project-notes.md。\n- そのままにするもの：コマンド、タスク ID、ファイルのパス、コード。\n- 訳文で元の文を置き換え、元の文は残しません。\n- タスクの言語（`alm task-language` が表示するもの）は変えません。\n"
        ),
        (_, false) => "Translation: do not translate. The taskbag's contents stay in the language they are written in.\n".to_string(),
        (_, true) => format!(
            "Translation: translate the taskbag's contents into {name}, after the work above, as part of the same plan, and only after the person chooses Go ahead.\n- Translate: every task's title, description and handoff note (`alm title`, `alm describe`, `alm handoff`), the card files (`tasks/<id>/card.md`), and the taskbag's project-notes.md.\n- Keep as they are: commands, task IDs, file paths and code.\n- The translation replaces the old text; the originals are not kept.\n- The task language (what `alm task-language` prints) does not change: leave it.\n"
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
        for (language, name) in [("en", "English"), ("zh-CN", "简体中文"), ("ja", "日本語")]
        {
            let yes = agent_update_taskbag_request_with(language, &p, &tb, true);
            assert!(yes.contains(name), "{language}: {yes}");
            for kept in [
                "alm task-language",
                "tasks/<id>/card.md",
                "project-notes.md",
            ] {
                assert!(yes.contains(kept), "{language}: {kept}");
            }
            let no = agent_update_taskbag_request_with(language, &p, &tb, false);
            assert!(
                !no.contains("tasks/<id>/card.md") || no.contains("move-cards"),
                "{language}"
            );
            assert!(!no.contains(name) || language == "en", "{language}: {no}");
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
