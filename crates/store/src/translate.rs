//! **Translate the taskbag…** (Task-A05-board831, approver 2026-10-26): what an AI agent is asked
//! to do to rewrite a taskbag's contents in another language and make it the task language. Any
//! language goes: it is named by its code (`vi`, `pt-BR`), with the name the person saw when the
//! app has one. `alm translate <code>` prints the same request, so `alm` and the app say the same
//! thing. [`translation_steps_in`] is the part Update the taskbag… shares (Task-A05-cape996).

use std::path::Path;

use crate::shown;

/// The target language as the request names it: "Tiếng Việt (`vi`)", or the code alone. Without a
/// name, the interface languages are named as people who speak them name them.
pub(crate) fn target_text(code: &str, name: Option<&str>) -> String {
    let known = match code {
        "en" => Some("English"),
        "zh-CN" => Some("简体中文"),
        "ja" => Some("日本語"),
        _ => None,
    };
    match name
        .map(str::trim)
        .filter(|n| !n.is_empty() && *n != code)
        .or(known)
    {
        Some(name) => format!("{name} (`{code}`)"),
        None => format!("`{code}`"),
    }
}

/// What to translate, what to keep, and the task language to set, as a list in the request's
/// language (`en`, `zh-CN`, `ja`; English otherwise). `taskbag` is named in the
/// `alm task-language` command.
pub fn translation_steps_in(
    language: &str,
    taskbag: &Path,
    code: &str,
    name: Option<&str>,
) -> String {
    let target = target_text(code, name);
    let bag = shown(taskbag);
    match language {
        "zh-CN" => format!(
            "- 翻译成{target}：每个任务的标题、描述和交接备注（`alm title`、`alm describe`、`alm handoff`），卡片文件（`tasks/<id>/card.md`），以及任务包的 project-notes.md。\n- 保持原样：命令、任务 ID、文件路径和代码。\n- 译文取代原文，不保留原文。\n- 同时把任务语言改成这种语言，以后的新任务也用它来写：`alm --taskbag \"{bag}\" task-language {code}`。\n"
        ),
        "ja" => format!(
            "- {target}に翻訳するもの：各タスクのタイトル、説明、引き継ぎメモ（`alm title`、`alm describe`、`alm handoff`）、カードのファイル（`tasks/<id>/card.md`）、タスクバッグの project-notes.md。\n- そのままにするもの：コマンド、タスク ID、ファイルのパス、コード。\n- 訳文で元の文を置き換え、元の文は残しません。\n- 同時にタスクの言語をこの言語に変え、これからの新しいタスクもこの言語で書かれるようにします：`alm --taskbag \"{bag}\" task-language {code}`。\n"
        ),
        _ => format!(
            "- Translate into {target}: every task's title, description and handoff note (`alm title`, `alm describe`, `alm handoff`), the card files (`tasks/<id>/card.md`), and the taskbag's project-notes.md.\n- Keep as they are: commands, task IDs, file paths and code.\n- The translation replaces the old text; the originals are not kept.\n- At the same time, make it the task language, so new tasks are written in it too: `alm --taskbag \"{bag}\" task-language {code}`.\n"
        ),
    }
}

/// The whole request: translate the taskbag at `taskbag` (whose project folder is `project`)
/// into `code`. The agent shows its plan in the app first (`alm confirm`) and changes nothing until
/// the person chooses Go ahead, as Update the taskbag… asks.
pub fn agent_translate_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    code: &str,
    name: Option<&str>,
) -> String {
    let target = target_text(code, name);
    let steps = translation_steps_in(language, taskbag, code, name);
    let (project, bag) = (shown(project), shown(taskbag));
    match language {
        "zh-CN" => format!(
            "请把这个任务包的内容翻译成{target}（Allomos 的“翻译任务包”）。\n\n项目文件夹：{project}\n任务包（Allomos 的任务文件）：{bag}\n\n先用 `alm --taskbag \"{bag}\" confirm` 在 Allomos 应用里给用户看你的计划（要翻译多少个任务和文件），用户选择“继续”之前不要改动任何东西。然后：\n\n{steps}\n每次改动后，按 `alm` 打印的命令用你的版本控制工具保存任务包里改动的文件；除非用户要求，不要共享（推送）。最后告诉用户你翻译了什么。\n"
        ),
        "ja" => format!(
            "このタスクバッグの内容を{target}に翻訳してください（Allomos の「タスクバッグを翻訳」）。\n\nプロジェクトのフォルダー：{project}\nタスクバッグ（Allomos のタスクファイル）：{bag}\n\nまず `alm --taskbag \"{bag}\" confirm` で Allomos アプリにユーザー向けの計画（翻訳するタスクとファイルの数）を見せ、ユーザーが「進める」を選ぶまで何も変えないでください。そのあと：\n\n{steps}\n変更のたびに、`alm` が表示するコマンドのとおりに、タスクバッグの変わったファイルをバージョン管理ツールで保存します。頼まれない限り共有（プッシュ）しません。最後に、何を翻訳したかをユーザーに伝えてください。\n"
        ),
        _ => format!(
            "Please translate this taskbag's contents into {target} (Allomos's Translate the taskbag).\n\nProject folder: {project}\nTaskbag (Allomos's task files): {bag}\n\nFirst show the person your plan in the Allomos app with `alm --taskbag \"{bag}\" confirm` (how many tasks and files you will translate), and change nothing until they choose Go ahead. Then:\n\n{steps}\nAfter each change, save the taskbag's changed files in your version control tool, as `alm` prints them; do not share (push) unless the person asks. At the end, tell the person what you translated.\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_names_the_language_and_sets_it() {
        let (p, b) = (Path::new("/p"), Path::new("/p/.allomos/taskbag"));
        let r = agent_translate_request_in("en", p, b, "vi", Some("Tiếng Việt"));
        assert!(r.contains("into Tiếng Việt (`vi`)"), "{r}");
        assert!(r.contains("task-language vi"), "{r}");
        assert!(r.contains("Keep as they are: commands, task IDs, file paths and code."));
        assert!(r.contains("confirm"), "{r}");
        // Without a name, the code alone; every language says the same.
        assert!(agent_translate_request_in("en", p, b, "pt-BR", None).contains("into `pt-BR`"));
        for language in ["zh-CN", "ja"] {
            let r = agent_translate_request_in(language, p, b, "vi", Some("Tiếng Việt"));
            assert!(r.contains("Tiếng Việt (`vi`)"), "{language}: {r}");
            assert!(r.contains("task-language vi"), "{language}: {r}");
        }
    }
}
