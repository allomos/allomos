//! The rules for a task's risk, as `alm classify <id>` (without options) and `alm done` list them
//! for the agent (Task-A01-steam844, decision 0027 § 3 and 4): the fixed ones (H1 to H7, M1 to
//! M4, low) and the project's own (`project.json`'s `risk`, `rules`; Task-A01-beetle749), in the
//! task language. `alm` tells; the agent judges: nothing here checks or refuses anything.

use std::path::Path;

use allomos_core::risk::Applied;
use allomos_store::{load, task_language, taskbag_of};
use serde_json::{Value, json};

/// The taskbag's task language and its risk settings as they apply; English and no rules of its
/// own when the taskbag cannot be read.
pub(crate) fn of_taskbag(root: &Path) -> (String, Applied) {
    let system: Vec<String> = sys_locale::get_locales().collect();
    taskbag_of(root, load(root))
        .map(|(bag, _)| {
            (
                task_language(&bag.project, &system).language,
                allomos_core::risk::applied(&bag.project),
            )
        })
        .unwrap_or_else(|_| ("en".to_string(), Applied::default()))
}

/// The fixed rules' short names, as the JSON output lists them.
const FIXED_MEDIUM: [&str; 4] = ["M1", "M2", "M3", "M4"];

struct Words {
    heading: &'static str,
    high: &'static str,
    h: [&'static str; 7],
    medium: &'static str,
    m: [&'static str; 4],
    low: &'static str,
    own: &'static str,
    own_none: &'static str,
    highest: &'static str,
}

const EN: Words = Words {
    heading: "Risk rules (decision 0027; alm lists them, you judge):",
    high: "High:",
    h: [
        "an outside effect: a hosted resource, SQL on a real target, a deployment, a secret, sending email, creating a repository, adding or changing a remote, other people's data.",
        "a release.",
        "a change of data format: the format of the files of tasks, events or the taskbag; a migration.",
        "removing something: a feature, a command, data, files.",
        "permissions and security: credentials, pairing, sign-in, signing, who may do what.",
        "a change of shared rules: AGENTS.md, `alm guide`.",
        "making `alm` do one more check of its own.",
    ],
    medium: "Medium:",
    m: [
        "a large change (say 500 lines or more, or 3 modules or more).",
        "a check that was not run, or finishing with `--without-human-check`.",
        "words and look that users see.",
        "the principle of how something is built (the kind of AI decision the approver reviews).",
    ],
    low: "Low: everything else: only documents, internal tidying or tests, with every check passed.",
    own: "This project's rules:",
    own_none: "This project's rules: none added.",
    highest: "When several apply, the highest one counts.",
};

const ZH_CN: Words = Words {
    heading: "风险规则（决定 0027；alm 列出，由你判断）：",
    high: "高：",
    h: [
        "有对外影响：托管资源、对真实目标执行 SQL、部署、密钥、发邮件、建仓库、加或改远程、碰别人的数据。",
        "发布。",
        "改数据格式：任务、事件、任务包的文件格式，或迁移。",
        "删掉东西：功能、命令、数据或文件。",
        "涉及权限和安全：凭据、配对、登录、签名、谁能做什么。",
        "改共享规则：AGENTS.md 或 `alm guide`。",
        "让 `alm` 自己多做一项检查。",
    ],
    medium: "中：",
    m: [
        "改动大（例如 500 行以上，或碰到 3 个以上模块）。",
        "有检查没跑，或用 `--without-human-check` 完成。",
        "用户看得到的文字和样子。",
        "实现方法的原理（批准者要审的那类 AI 决定）。",
    ],
    low: "低：其他的：只改文档、内部整理、测试，而且检查都过了。",
    own: "这个项目加的规则：",
    own_none: "这个项目加的规则：没有。",
    highest: "符合几条时，按最高的一条算。",
};

const JA: Words = Words {
    heading: "リスクのルール（決定 0027。alm は並べるだけ、判断するのはあなた）：",
    high: "高：",
    h: [
        "外部への影響がある：ホストされたリソース、実際の対象への SQL 実行、デプロイ、シークレット、メール送信、リポジトリ作成、リモートの追加や変更、他人のデータに触れること。",
        "リリース。",
        "データ形式を変える：タスク、イベント、タスクバッグのファイル形式、または移行。",
        "何かを削除する：機能、コマンド、データ、ファイル。",
        "権限とセキュリティに関わる：認証情報、ペアリング、サインイン、署名、誰が何をできるか。",
        "共有ルールを変える：AGENTS.md または `alm guide`。",
        "`alm` 自身が行うチェックを一つ増やす。",
    ],
    medium: "中：",
    m: [
        "大きな変更（たとえば 500 行以上、または 3 つ以上のモジュール）。",
        "実行していないチェックがある、または `--without-human-check` で完了する。",
        "利用者に見える文言と見た目。",
        "作り方の原則（承認者が見直す種類の AI の判断）。",
    ],
    low: "低：それ以外：文書だけ、内部の整理、テストだけで、チェックがすべて通ったもの。",
    own: "このプロジェクトが追加したルール：",
    own_none: "このプロジェクトが追加したルール：なし。",
    highest: "いくつも当てはまるときは、いちばん高いものに従います。",
};

fn words(language: &str) -> &'static Words {
    match language {
        "zh-CN" => &ZH_CN,
        "ja" => &JA,
        _ => &EN,
    }
}

/// The rules as text, in `language` (`en`, `zh-CN`, `ja`; English otherwise).
pub(crate) fn text(language: &str, applied: &Applied) -> String {
    let w = words(language);
    let mut out = format!("{}\n  {}\n", w.heading, w.high);
    for (id, rule) in allomos_core::risk::FIXED_HIGH.iter().zip(w.h) {
        out.push_str(&format!("    {id} {rule}\n"));
    }
    out.push_str(&format!("  {}\n", w.medium));
    for (id, rule) in FIXED_MEDIUM.iter().zip(w.m) {
        out.push_str(&format!("    {id} {rule}\n"));
    }
    out.push_str(&format!("  {}\n", w.low));
    if applied.rules.is_empty() {
        out.push_str(&format!("  {}\n", w.own_none));
    } else {
        out.push_str(&format!("  {}\n", w.own));
        for rule in &applied.rules {
            out.push_str(&format!("    - {rule}\n"));
        }
    }
    out.push_str(&format!("  {}\n", w.highest));
    out
}

/// What `alm done` says when the task has no risk recorded: one sentence and the command.
pub(crate) fn not_classified(language: &str, id: &str) -> String {
    let command = format!(
        "alm classify {id} --risk low|medium|high --touches production|others-data|none --why"
    );
    match language {
        "zh-CN" => format!(
            "这个任务还没分级（alm 提醒，不拦着）：按下面的规则判断，用 `{command} \"<一句话：哪几条规则适用，工作里的什么让它们适用>\"` 记下。\n"
        ),
        "ja" => format!(
            "このタスクはまだリスクが分類されていません（alm は知らせるだけで、止めません）：下のルールで判断し、`{command} \"<一文で：どのルールが当てはまり、作業の何がそうさせるか>\"` で記録します。\n"
        ),
        _ => format!(
            "This task has no risk recorded yet (alm reminds; it stops nothing): judge it by the rules below and record it with `{command} \"<one sentence: which rules apply, and what in the work makes them apply>\"`.\n"
        ),
    }
}

/// The rules for the JSON output: the fixed ones by their short names, the project's own as
/// written.
pub(crate) fn json(applied: &Applied) -> Value {
    json!({
        "fixed_high": allomos_core::risk::FIXED_HIGH,
        "fixed_medium": FIXED_MEDIUM,
        "project_rules": applied.rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_lists_the_fixed_rules_and_the_projects_own() {
        let applied = Applied {
            rules: vec!["Changing the price list is high risk.".to_string()],
            ..Default::default()
        };
        for language in ["en", "zh-CN", "ja", "fr"] {
            let t = text(language, &applied);
            for id in ["H1", "H7", "M1", "M4"] {
                assert!(t.contains(&format!("    {id} ")), "{language}: {id}");
            }
            assert!(t.contains("    - Changing the price list is high risk."));
        }
        assert!(text("zh-CN", &Applied::default()).contains("这个项目加的规则：没有。"));
    }

    #[test]
    fn the_reminder_gives_the_command() {
        for language in ["en", "zh-CN", "ja"] {
            assert!(
                not_classified(language, "A01-x1")
                    .contains("alm classify A01-x1 --risk low|medium|high")
            );
        }
    }
}
